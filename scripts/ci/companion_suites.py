#!/usr/bin/env python3
"""Run a package's companion suites for one cell and record each one's outcome.

A companion suite is a non-Cargo suite a Cargo package owns. `_package-ci.yml`
used to carry one hard-coded step per suite and record a single `companion`
string, so one suite's success satisfied every suite the package declared. This
runner replaces that: it reads the declared registry, runs every suite attached
to the `{environment, gate}` cell it was given, and writes ONE record per suite
— outcome, counts, and command duration.

Counts come from each suite's own machine-readable output (`suite_runner.py`'s
document, Vitest's `--reporter=json`). A suite with no test cardinality to
report — `tsc --noEmit` is the concrete case — records `counts: null` with the
registry's reason, never a `0` that reads as evidence (AC13).

## Examples

```bash
python3 scripts/ci/companion_suites.py \
    --suites '["test_schema.py"]' --environment ubuntu-latest \
    --gate L1 --out "$RUNNER_TEMP/companions.json"
```

## Exit status

`0` when every attached suite succeeded, `1` when any failed, and `2` when the
runner could not read its own inputs.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import affected_scope  # noqa: E402  (needs the path insert above)
import schema  # noqa: E402

#: What a counts document is read as, keyed by the registry's strategy name.
#: Vitest reports pending tests as its skip count and has no separate error
#: bucket; a failed-to-run file surfaces as a failed test.
COUNTS_READERS = {
    "json": lambda document: {
        field: int(document.get(field, 0)) for field in schema.COUNT_FIELDS
    },
    "vitest": lambda document: {
        "total": int(document.get("numTotalTests", 0)),
        "passed": int(document.get("numPassedTests", 0)),
        "failed": int(document.get("numFailedTests", 0)),
        "skipped": int(document.get("numPendingTests", 0)),
        "errored": 0,
    },
}


def read_counts(strategy: str, path: Path) -> tuple[dict[str, int] | None, str]:
    """The counts a suite recorded, or `None` with the reason it did not.

    ## Returns

    `(counts, reason)`. Exactly one is meaningful: a suite that recorded counts
    carries an empty reason, and a suite that did not carries no counts and a
    reason naming what was missing.
    """
    reader = COUNTS_READERS.get(strategy)
    if reader is None:
        return None, f"unknown counts strategy '{strategy}'"
    if not path.is_file():
        return None, "the suite recorded no counts document"
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return None, f"the suite's counts document could not be read: {error}"
    try:
        return reader(document), ""
    except (AttributeError, TypeError, ValueError) as error:
        return None, f"the suite's counts document had an unexpected shape: {error}"


def run_one(record: dict[str, Any], root: Path, scratch: Path) -> dict[str, Any]:
    """Run one attached suite and return its outcome record.

    Output is inherited rather than captured: the suite's own log is the
    reviewer's first stop when it fails, and burying it behind this runner
    would trade a measurement for a diagnosis.
    """
    command = record["recipe"]
    strategy = record.get("counts")
    counts_path = scratch / f"{record['name']}.counts.json"
    if strategy:
        command = (
            f"{command} "
            + record["counts_args"].replace(
                affected_scope.COUNTS_OUT_PLACEHOLDER, str(counts_path)
            )
        )

    # Foldable in the CI log, plain everywhere else: the markers are literal
    # noise in a local run and in this runner's own test output.
    folding = bool(os.environ.get("GITHUB_ACTIONS"))
    if folding:
        print(f"::group::companion suite {record['name']}", flush=True)
    print(f"$ {command}", flush=True)
    # A recipe may open with a relative `cd` (`cd tools/test-toolkit && …`). With
    # `CDPATH` set and no `.` entry in it, a POSIX shell resolves that against
    # `CDPATH` *before* the working directory, so from a worktree the recipe runs
    # in whichever other checkout `CDPATH` names. `cwd=root` is the contract;
    # the caller's interactive search path must not be able to override it.
    environment = {key: value for key, value in os.environ.items() if key != "CDPATH"}
    started = time.monotonic()
    completed = subprocess.run(
        command, shell=True, cwd=root, env=environment, check=False
    )
    duration = time.monotonic() - started
    if folding:
        print("::endgroup::", flush=True)

    outcome = "success" if completed.returncode == 0 else "failure"
    if strategy:
        counts, reason = read_counts(strategy, counts_path)
    else:
        counts, reason = None, record.get("counts_reason", "")

    result: dict[str, Any] = {
        "outcome": outcome,
        "duration_s": round(duration, 3),
    }
    if counts is None:
        result["reason"] = reason or "the producer recorded no counts"
    else:
        result["counts"] = counts
    measurement = (
        f"{counts['total']} test(s), {counts['failed'] + counts['errored']} failed"
        if counts is not None
        else f"counts not recorded ({result['reason']})"
    )
    print(
        f"companion suite {record['name']}: {outcome}, {measurement}, "
        f"{result['duration_s']}s",
        flush=True,
    )
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suites", required=True, help="JSON array of declared suite names")
    parser.add_argument("--environment", required=True, help="the cell's environment")
    parser.add_argument("--gate", required=True, help="the cell's gate (`L1` or `lint`)")
    parser.add_argument("--out", type=Path, required=True, help="where to write the records")
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="repository root")
    args = parser.parse_args(argv)

    try:
        names = json.loads(args.suites)
    except ValueError as error:
        print(f"--suites is not JSON: {error}", file=sys.stderr)
        return 2
    if not isinstance(names, list) or not all(isinstance(name, str) for name in names):
        print("--suites must be a JSON array of suite names", file=sys.stderr)
        return 2

    unknown = [name for name in names if name not in affected_scope.SUITE_REGISTRY]
    if unknown:
        print(
            f"unregistered companion suite(s) {unknown}; registered: "
            f"{sorted(affected_scope.SUITE_REGISTRY)}",
            file=sys.stderr,
        )
        return 2

    records = affected_scope.companion_records(names, args.environment, args.gate)
    results: dict[str, Any] = {}
    with tempfile.TemporaryDirectory(prefix="companion-counts-") as scratch:
        for record in records:
            results[record["name"]] = run_one(record, args.root, Path(scratch))

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")

    failed = sorted(
        name for name, result in results.items() if result["outcome"] != "success"
    )
    if failed:
        print(f"companion suite(s) FAILED: {', '.join(failed)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

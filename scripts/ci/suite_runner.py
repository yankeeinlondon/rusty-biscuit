#!/usr/bin/env python3
"""Run one Python contract suite and record machine-readable counts.

Every suite in `scripts/ci/` ends with `unittest.main(verbosity=2)` and reports
its result as prose on stderr (`Ran 130 tests in 11.503s` / `OK`). That prose is
a drift surface: a wording change in the standard library silently turns a
measured suite into an unmeasured one. This runner loads the same module and
runs it under its own `TestResult`, so the counts come from the objects unittest
already built rather than from re-reading its output (S3).

Running a suite directly still works and is unchanged; it simply reports no
counts. This wrapper is what CI runs, and what the suite registry names as the
canonical recipe, so the command in the registry is the command a developer can
reproduce.

## Examples

```bash
python3 scripts/ci/suite_runner.py test_schema.py
python3 scripts/ci/suite_runner.py test_schema.py --counts-out /tmp/counts.json
```
"""

from __future__ import annotations

import argparse
import importlib
import json
import sys
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))


def counts_of(result: unittest.TestResult) -> dict[str, int]:
    """The five-field count document `schema.COUNT_FIELDS` names.

    `testsRun` includes skipped tests, so `passed` is what is left after every
    non-passing outcome is removed — an unexpected success included, because a
    test that was expected to fail and did not is not a pass.
    """
    failed = len(result.failures)
    errored = len(result.errors)
    skipped = len(result.skipped)
    unexpected = len(result.unexpectedSuccesses)
    return {
        "total": result.testsRun,
        "passed": max(result.testsRun - failed - errored - skipped - unexpected, 0),
        "failed": failed + unexpected,
        "skipped": skipped,
        "errored": errored,
    }


def run_suite(module_name: str) -> tuple[unittest.TestResult, float]:
    """Load `module_name` from this directory and run every test it defines."""
    module = importlib.import_module(module_name)
    tests = unittest.TestLoader().loadTestsFromModule(module)
    runner = unittest.TextTestRunner(verbosity=2)
    started = time.monotonic()
    result = runner.run(tests)
    return result, time.monotonic() - started


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "suite",
        help="suite module in scripts/ci/, with or without its .py suffix",
    )
    parser.add_argument(
        "--counts-out",
        type=Path,
        help="write the counts document here; omitted, the suite reports counts nowhere",
    )
    args = parser.parse_args(argv)

    module_name = args.suite.removesuffix(".py")
    result, duration = run_suite(module_name)

    if args.counts_out is not None:
        document = {**counts_of(result), "duration_s": round(duration, 3)}
        args.counts_out.parent.mkdir(parents=True, exist_ok=True)
        args.counts_out.write_text(
            json.dumps(document, indent=2) + "\n", encoding="utf-8"
        )

    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Run every binary-distribution Python suite as one companion suite.

This is what `just distribution-test` runs inside its temporary pinned Python
environment, and what CI runs as `repo-deps`'s `distribution` companion suite
(`SUITE_REGISTRY` in `scripts/ci/affected_scope.py`).

Two rules make a green run mean the suites ran:

* the interpreter must carry the exact versions pinned in
  `scripts/ci/requirements-distribution.txt`; a host interpreter with no PyYAML,
  or a different one, fails before any test loads;
* a skipped test fails the run. These suites check real tools (`shellcheck`,
  `ruby`, `gpg`, `shasum`, `cargo`), and a skip under a green companion cell
  would claim coverage that never happened.

## Examples

```bash
just distribution-test
just distribution-test --counts-out /tmp/counts.json
```
"""

from __future__ import annotations

import argparse
import importlib
import importlib.metadata
import json
import sys
import time
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
CI_DIR = HERE.parent / "ci"
REQUIREMENTS = CI_DIR / "requirements-distribution.txt"
if str(CI_DIR) not in sys.path:
    sys.path.insert(0, str(CI_DIR))

# The counts document every companion suite writes; skips are counted, and a
# skip also fails the run (see `main`).
from suite_runner import counts_of  # noqa: E402

# Every suite, by directory. `unlisted_suites` refuses a `test_*.py` added to
# `scripts/distribution/` without an entry here, so a new suite cannot sit
# beside the others and never run.
SUITES: tuple[tuple[Path, str], ...] = (
    (CI_DIR, "test_distribution"),
    (HERE, "test_inspect"),
    (HERE, "test_installers"),
    (HERE, "test_licenses"),
    (HERE, "test_tap"),
    (HERE, "test_verify_binstall"),
    (HERE, "test_verify_runtime"),
)


class SetupError(Exception):
    """The environment cannot run the suites; nothing was run."""


def pinned_requirements(text: str) -> dict[str, str]:
    """`name==version` lines from the requirements file, comments ignored.

    ## Errors

    `SetupError` for any line that is not an exact `==` pin: an unpinned
    requirement would let two runs test against different libraries.
    """
    pins: dict[str, str] = {}
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        name, separator, version = line.partition("==")
        if not separator or not name.strip() or not version.strip():
            raise SetupError(f"{REQUIREMENTS.name}:{number}: {raw.strip()!r} is not an exact `name==version` pin")
        pins[name.strip()] = version.strip()
    if not pins:
        raise SetupError(f"{REQUIREMENTS.name} pins nothing")
    return pins


def check_environment(pins: dict[str, str]) -> None:
    """Refuse an interpreter that does not carry exactly the pinned versions."""
    problems = []
    for name, version in pins.items():
        try:
            installed = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError:
            problems.append(f"{name} is not installed (pinned {version})")
            continue
        if installed != version:
            problems.append(f"{name} {installed} is installed, but {version} is pinned")
    if problems:
        raise SetupError(
            "this interpreter cannot run the distribution suites: "
            + "; ".join(problems)
            + ". Run them through `just distribution-test`, which provisions the pinned environment."
        )


def unlisted_suites() -> list[str]:
    listed = {name for directory, name in SUITES if directory == HERE}
    return sorted(path.stem for path in HERE.glob("test_*.py") if path.stem not in listed)


def load_suites() -> unittest.TestSuite:
    if str(HERE) not in sys.path:
        sys.path.insert(0, str(HERE))
    loader = unittest.TestLoader()
    combined = unittest.TestSuite()
    for _, name in SUITES:
        combined.addTests(loader.loadTestsFromModule(importlib.import_module(name)))
    return combined


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--counts-out", type=Path, help="write the counts document here")
    args = parser.parse_args(argv)

    try:
        check_environment(pinned_requirements(REQUIREMENTS.read_text(encoding="utf-8")))
        unlisted = unlisted_suites()
        if unlisted:
            raise SetupError(f"suites not listed in run_suites.SUITES: {', '.join(unlisted)}")
    except SetupError as error:
        print(f"run_suites: {error}", file=sys.stderr)
        return 2

    started = time.monotonic()
    result = unittest.TextTestRunner(verbosity=2).run(load_suites())
    duration = time.monotonic() - started

    if args.counts_out is not None:
        document = {**counts_of(result), "duration_s": round(duration, 3)}
        args.counts_out.parent.mkdir(parents=True, exist_ok=True)
        args.counts_out.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")

    if result.skipped:
        print("run_suites: a skipped test fails this run; provide what it needs:", file=sys.stderr)
        for test, reason in result.skipped:
            print(f"  {test.id()}: {reason}", file=sys.stderr)
        return 1
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Decide, before downloading a cell's build archive, whether its owner delivered it.

The build owner publishes one `build-status-{package}-{producer}-{key}` artifact
per planned build record, holding `build-status.json`. A consumer downloads that
artifact first (a failed download is tolerated there and only there) and runs
this check: only a valid status reporting `success` for exactly this cell's
record lets the archive download proceed. Everything else refuses the cell
before any archive or test is touched, with the producer named, so a known
failed or unknown build reads as `not run` rather than as a broken download.

A successful status does not prove the archive exists. If the download that
follows finds no archive, that is an artifact-publication failure and fails the
step as before; nothing here compiles, fabricates a report, or writes a
completion record.

Python 3 stdlib only, and runs on the native Windows host of the WSL2 leg as
well as on every native consumer, so file I/O is UTF-8 explicitly.

## Examples

```bash
python3 scripts/ci/build_status.py \
    --status "$RUNNER_TEMP/build-status/build-status.json" \
    --package sniff-detection-cli \
    --key abc0123456789def \
    --producer ubuntu-latest \
    --artifact build-sniff-detection-cli-ubuntu-latest-abc0123456789def
```

## Exit status

`0` only for a valid status whose identity matches the cell and whose result is
`success`. `1` for a refusal (failed, cancelled, or absent status) and for an
invalid document, which is reported as `{file}: {field}: {problem}`.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

from schema import is_version, loads_strict  # noqa: E402

BUILD_STATUS_SCHEMA_VERSION = 1
RESULTS = ("success", "failure", "cancelled")
#: The fields that must equal the resolved cell's build record.
IDENTITY_FIELDS = ("package", "key", "producer", "artifact")
#: `stage` is diagnostic: it is required, but never decides the outcome.
REQUIRED_STRINGS = (*IDENTITY_FIELDS, "result", "stage")


class StatusError(Exception):
    """An unusable build-status document, located at `file` and `field`."""

    def __init__(self, file: Path, field: str, problem: str) -> None:
        super().__init__(f"{file}: {field}: {problem}")
        self.file = file
        self.field = field
        self.problem = problem


@dataclass(frozen=True)
class Cell:
    """The build record a consuming cell was resolved to."""

    package: str
    key: str
    producer: str
    artifact: str

    @property
    def status_artifact(self) -> str:
        return f"build-status-{self.package}-{self.producer}-{self.key}"

    @property
    def owner(self) -> str:
        return f"build ({self.producer})"


@dataclass(frozen=True)
class Verdict:
    """Whether the archive download may proceed.

    `detail` is the status's own diagnostic text: `None` when the document
    carried none (absent or null), otherwise the string as written, including
    an empty one. It is always `None` for an absent status.
    """

    ok: bool
    message: str
    detail: str | None = None


def required_string(document: dict[str, Any], field: str, file: Path) -> str:
    if field not in document:
        raise StatusError(file, field, "missing")
    value = document[field]
    if value is None:
        raise StatusError(file, field, "null; expected a nonempty string")
    if not isinstance(value, str):
        raise StatusError(file, field, f"expected a nonempty string, found {type(value).__name__}")
    if value == "":
        raise StatusError(file, field, "empty; expected a nonempty string")
    return value


def optional_detail(document: dict[str, Any], file: Path) -> str | None:
    if "detail" not in document or document["detail"] is None:
        return None
    value = document["detail"]
    if not isinstance(value, str):
        raise StatusError(file, "detail", f"expected a string or null, found {type(value).__name__}")
    return value


def parse(text: str, file: Path, cell: Cell) -> Verdict:
    """Judge one build-status document against `cell`.

    ## Errors

    Raises `StatusError` for anything that is not a version-1 status for
    exactly this record: invalid JSON, a missing or mistyped required field, an
    identity that names another record, or a result outside `RESULTS`.
    """
    try:
        document = loads_strict(text)
    except ValueError as error:
        raise StatusError(file, "(document)", f"not one strict JSON document: {error}") from error
    if not isinstance(document, dict):
        raise StatusError(file, "(document)", f"expected an object, found {type(document).__name__}")
    if "schema_version" not in document:
        raise StatusError(file, "schema_version", "missing")
    if not is_version(document["schema_version"], BUILD_STATUS_SCHEMA_VERSION):
        raise StatusError(
            file,
            "schema_version",
            f"expected the integer {BUILD_STATUS_SCHEMA_VERSION}, "
            f"found {json.dumps(document['schema_version'])}",
        )
    fields = {field: required_string(document, field, file) for field in REQUIRED_STRINGS}
    detail = optional_detail(document, file)
    for field in IDENTITY_FIELDS:
        expected = getattr(cell, field)
        if fields[field] != expected:
            raise StatusError(
                file,
                field,
                f"names {fields[field]!r}, but this cell consumes {expected!r}",
            )
    result = fields["result"]
    if result not in RESULTS:
        raise StatusError(file, "result", f"unknown result {result!r}; expected one of {', '.join(RESULTS)}")
    if result == "success":
        return Verdict(True, f"build {cell.key} for {cell.package} on {cell.producer} succeeded", detail)
    message = (
        f"not run: producer failed: build {cell.key} for {cell.package} on {cell.producer} "
        f"concluded {result} at the {fields['stage']} stage"
    )
    if detail is not None:
        message += f"; detail: {json.dumps(detail)}"
    return Verdict(False, message, detail)


def check(status: Path, cell: Cell) -> Verdict:
    """Read `status` (the downloaded `build-status.json`) and judge it.

    A file that does not exist is an absent status: the owner's outcome for this
    record is unknown, which refuses the cell without claiming it failed.

    ## Errors

    Raises `StatusError` for an unreadable or invalid document (see `parse`).
    """
    if not status.exists():
        return Verdict(
            False,
            f"not run: producer outcome unknown: no {cell.status_artifact} status was "
            f"published for build {cell.key} of {cell.package}; its owner is the "
            f"{cell.producer} producer, job `{cell.owner}`",
        )
    try:
        text = status.read_bytes().decode("utf-8")
    except OSError as error:
        raise StatusError(status, "(document)", f"unreadable: {error}") from error
    except UnicodeDecodeError as error:
        raise StatusError(status, "(document)", f"not UTF-8: {error}") from error
    return parse(text, status, cell)


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--status", type=Path, required=True, help="the downloaded build-status.json")
    for field in IDENTITY_FIELDS:
        parser.add_argument(f"--{field}", required=True, help=f"the resolved cell's build {field}")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", errors="backslashreplace")
    args = parse_args(argv)
    empty = [field for field in IDENTITY_FIELDS if getattr(args, field) == ""]
    if empty:
        # An unresolved cell contract renders as empty strings; checking
        # against them would refuse with a misleading identity mismatch.
        print(f"::error::build_status: empty cell identity --{', --'.join(empty)}", file=sys.stderr)
        return 1
    cell = Cell(package=args.package, key=args.key, producer=args.producer, artifact=args.artifact)
    try:
        verdict = check(args.status, cell)
    except StatusError as error:
        print(f"::error::build_status: {error}", file=sys.stderr)
        return 1
    if not verdict.ok:
        print(f"::error::{verdict.message}", file=sys.stderr)
        return 1
    print(verdict.message)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

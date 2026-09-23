#!/usr/bin/env python3
"""The planner's path arguments, derived from one `git diff --name-status -z`.

Every selection boundary — `.github/workflows/ci.yml`, `just/ci-local.just`,
and `.githooks/pre-push` — feeds `affected_scope.py` the same two lists, and a
name-only diff cannot produce the second of them: it prints a removed path and
a modified path identically, so `change_inventory.deleted` stayed empty and the
archive-path guard had to guess whether an absent file was deleted or merely
missing. One parser lives here rather than three in three shells, one of which
(`.githooks/pre-push`) is POSIX `sh` and cannot read NUL-delimited records at
all.

Input is the raw `--name-status -z` stream on stdin. Output is the argument
tail the planner expects, NUL-delimited so a path containing a newline or a
space survives `xargs -0` and Bash's `read -r -d ''`::

    --deleted<NUL>gone.rs<NUL>--renamed-from<NUL>old.md<NUL>--<NUL>gone.rs<NUL>new.md<NUL>

The trailing `--` is always emitted, so an empty diff still produces a
well-formed (and non-empty) argument list.

## Notes

`--name-status -z` emits the status and each path as *separate* NUL-terminated
records, and a rename or copy emits two paths — source then destination — for
its one status. The destination is what the changed list carries, matching
`--name-only`, which prints the destination alone. A rename's source is not a
deletion — the planner documents `deleted` as a subset of the changed paths,
and the archive guard's exemption maintenance already reads the full tree — so
it travels separately as `--renamed-from`, whose only reader is the test-input
search: a test still naming a file that moved away must be selected. A copy
leaves its source in place and reports nothing extra.

Positional arguments are extra changed paths that no diff reports — the
untracked files `just ci-local` unions in. They are never deletions.
"""

from __future__ import annotations

import os
import sys

#: Statuses whose record carries two paths: source then destination.
TWO_PATH_STATUSES = (b"R", b"C")

#: The status of a removed path.
DELETED_STATUS = b"D"

#: The status whose source no longer exists afterwards (a copy's still does).
RENAMED_STATUS = b"R"


def parse_name_status(stream: bytes) -> tuple[list[bytes], list[bytes], list[bytes]]:
    """The changed, deleted, and renamed-away path lists of one `--name-status -z` stream.

    ## Returns

    `(changed, deleted, renamed_from)`, each in the order Git reported it.
    `deleted` is a subset of `changed`; `renamed_from` is disjoint from it.

    ## Errors

    Raises `ValueError` when a status record has no path, or a rename or copy
    has no destination. A truncated diff is a broken boundary, not an empty
    change set.
    """
    records = stream.split(b"\0")
    if records and records[-1] == b"":
        records.pop()

    changed: list[bytes] = []
    deleted: list[bytes] = []
    renamed_from: list[bytes] = []
    index = 0
    while index < len(records):
        status = records[index]
        index += 1
        if index >= len(records):
            raise ValueError(
                f"status {status.decode(errors='replace')!r} has no path; the "
                f"diff stream is truncated"
            )
        path = records[index]
        index += 1
        if status[:1] in TWO_PATH_STATUSES:
            if index >= len(records):
                raise ValueError(
                    f"status {status.decode(errors='replace')!r} names "
                    f"{path.decode(errors='replace')!r} but has no destination"
                )
            if status[:1] == RENAMED_STATUS:
                renamed_from.append(path)
            path = records[index]
            index += 1
        changed.append(path)
        if status[:1] == DELETED_STATUS:
            deleted.append(path)
    return changed, deleted, renamed_from


def arguments(
    changed: list[bytes], deleted: list[bytes], renamed_from: list[bytes] = ()
) -> bytes:
    """The NUL-delimited planner argument tail for these lists."""
    parts: list[bytes] = []
    for path in deleted:
        parts.extend((b"--deleted", path))
    for path in renamed_from:
        parts.extend((b"--renamed-from", path))
    parts.append(b"--")
    parts.extend(changed)
    return b"".join(part + b"\0" for part in parts)


def main() -> int:
    extra = [os.fsencode(argument) for argument in sys.argv[1:]]
    try:
        changed, deleted, renamed_from = parse_name_status(sys.stdin.buffer.read())
    except ValueError as error:
        print(f"diff_scope: {error}", file=sys.stderr)
        return 2
    sys.stdout.buffer.write(arguments(changed + extra, deleted, renamed_from))
    sys.stdout.buffer.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())

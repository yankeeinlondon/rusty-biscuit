#!/usr/bin/env python3
"""Targeted, line-oriented TOML merge for kache host configuration.

Sets exactly the requested keys inside one existing (or appended) table of a
TOML file while preserving every other key and comment — this host's kache
config carries hand-tuned caps, event-log sizing, and GC policy with
rationale comments, and clobbering them is hostile (spec §2 of
fixes/2026-09-23-ensuring-kache-support). No comment-preserving TOML writer
is available (python3's tomllib only parses), so the edit is line-oriented
and confined to the target table:

  * an existing `key =` line in the table is replaced in place;
  * a missing key is inserted at the table's end;
  * the table itself is appended when the file has none.

The result is written to a temporary file, VALIDATED by parsing it with
tomllib (the whole file parses and each requested key holds its intended
value), and only then renamed over the original, keeping a dated backup of
the previous file when its content actually changed. A validation failure
exits non-zero and leaves the original untouched — that contract feeds the
`just init` failure contract (spec §4).

Values: `true` and `false` (exactly) become TOML booleans; anything else is
written as a quoted string. Line-oriented also means line-shaped: a
header-like line inside a multi-line string would confuse the table scan —
kache and Cargo configs do not use multi-line strings, and the tomllib gate
catches anything that garbles the parse.

Usage:
    kache-config-merge.py FILE TABLE KEY VALUE [KEY VALUE ...]

Examples:
    kache-config-merge.py ~/.config/kache/config.toml cache \\
        local_store /Volumes/coding/kache ignore_env true
    kache-config-merge.py "$CARGO_HOME/config.toml" build rustc-wrapper kache
"""

import datetime
import os
import pathlib
import re
import sys
import tempfile
import tomllib

IDENT = re.compile(r"^[A-Za-z0-9_-]+$")
TABLE_HEADER = re.compile(r"^\s*(\[+)([^\]]*?)(\]+)\s*(?:#.*)?$")


def fail(message: str) -> None:
    print(f"kache-config-merge: error={message}", file=sys.stderr)
    sys.exit(1)


def toml_value(raw: str) -> object:
    if raw == "true":
        return True
    if raw == "false":
        return False
    return raw


def render_value(raw: str) -> str:
    if raw == "true":
        return "true"
    if raw == "false":
        return "false"
    escaped = raw.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def table_range(lines: list[str], table: str) -> tuple[int, int] | None:
    """The half-open [header, end) line range of the exact `[table]` header.

    Dotted headers (`[a.b]`), array-of-table headers (`[[x]]`), and other
    tables are all boundaries, never matches; a key of the same name under a
    different table is not touched.
    """
    header = None
    for index, line in enumerate(lines):
        match = TABLE_HEADER.match(line)
        if not match:
            continue
        is_plain_header = match.group(1) == "[" and match.group(3) == "]"
        if is_plain_header and match.group(2).strip() == table:
            if header is None:
                header = index
                continue
        if header is not None:
            return (header, index)
    if header is None:
        return None
    return (header, len(lines))


def insert_at_table_end(lines: list[str], start: int, end: int) -> int:
    """Where a missing key line goes: the table's end, before any blank lines
    that separate it from the next table (those separators belong to the
    boundary, not to the table's content)."""
    index = end
    while index > start + 1 and lines[index - 1].strip() == "":
        index -= 1
    return index


def merge(lines: list[str], table: str, pairs: list[tuple[str, str]]) -> list[str]:
    scope = table_range(lines, table)
    if scope is None:
        result = list(lines)
        if result and result[-1].strip() != "":
            result.append("")
        result.append(f"[{table}]")
        for key, raw in pairs:
            result.append(f"{key} = {render_value(raw)}")
        return result

    start, end = scope
    result = list(lines)
    for key, raw in pairs:
        pattern = re.compile(rf"^\s*{re.escape(key)}\s*=")
        for index in range(start + 1, end):
            if pattern.match(result[index]):
                result[index] = f"{key} = {render_value(raw)}"
                break
        else:
            result.insert(
                insert_at_table_end(result, start, end), f"{key} = {render_value(raw)}"
            )
            end += 1
    return result


def validate(path: pathlib.Path, table: str, pairs: list[tuple[str, str]]) -> None:
    try:
        with open(path, "rb") as handle:
            data = tomllib.load(handle)
    except tomllib.TOMLDecodeError as error:
        fail(f"validation: {path} does not parse: {error}")
    section = data.get(table)
    if not isinstance(section, dict):
        fail(f"validation: [{table}] is not a table in {path}")
    for key, raw in pairs:
        want = toml_value(raw)
        got = section.get(key)
        if got != want or type(got) is not type(want):
            fail(f"validation: {table}.{key} holds {got!r}, expected {want!r}")


def main(argv: list[str]) -> None:
    # FILE TABLE KEY VALUE [KEY VALUE ...]: the script name plus three
    # leading words plus key/value pairs — always an odd count, at least 5.
    if len(argv) < 5 or len(argv) % 2 == 0:
        print(__doc__.strip(), file=sys.stderr)
        sys.exit(2)
    path = pathlib.Path(argv[1])
    table = argv[2]
    if not IDENT.match(table):
        fail(f"table name {table!r} is not a bare TOML key")
    pairs: list[tuple[str, str]] = []
    rest = argv[3:]
    for index in range(0, len(rest), 2):
        key, raw = rest[index], rest[index + 1]
        if not IDENT.match(key):
            fail(f"key name {key!r} is not a bare TOML key")
        pairs.append((key, raw))

    try:
        original_bytes = path.read_bytes() if path.exists() else None
    except OSError as error:
        fail(f"cannot read {path}: {error}")
    if original_bytes is not None:
        try:
            text = original_bytes.decode("utf-8")
        except UnicodeDecodeError as error:
            fail(f"{path} is not valid UTF-8: {error}")
        newline = "\r\n" if "\r\n" in text else "\n"
        lines = text.split("\n")
        had_trailing_newline = text.endswith("\n")
        if lines and lines[-1] == "":
            lines = lines[:-1]
        lines = [line[:-1] if line.endswith("\r") else line for line in lines]
    else:
        newline = "\n"
        lines = []
        had_trailing_newline = True

    merged = merge(lines, table, pairs)
    body = "\n".join(merged)
    if had_trailing_newline or original_bytes is None:
        body += "\n"
    merged_bytes = body.replace("\n", newline).encode("utf-8")

    if original_bytes == merged_bytes:
        keys = ", ".join(f"{table}.{key}" for key, _ in pairs)
        print(f"kache-config-merge: {path}: already holds {keys}")
        return

    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        descriptor, temp_name = tempfile.mkstemp(
            dir=str(path.parent), prefix=path.name, suffix=".merge-tmp"
        )
    except OSError as error:
        fail(f"cannot write beside {path}: {error}")
    temp_path = pathlib.Path(temp_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(merged_bytes)
        validate(temp_path, table, pairs)
        if original_bytes is not None:
            stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
            backup = path.with_name(f"{path.name}.bak-{stamp}")
            collision = 2
            while backup.exists():
                backup = path.with_name(f"{path.name}.bak-{stamp}-{collision}")
                collision += 1
            backup.write_bytes(original_bytes)
            os.replace(temp_path, path)
            keys = ", ".join(key for key, _ in pairs)
            print(
                f"kache-config-merge: {path}: wrote {keys} into [{table}]"
                f" (backup: {backup})"
            )
        else:
            os.replace(temp_path, path)
            keys = ", ".join(key for key, _ in pairs)
            print(f"kache-config-merge: {path}: created with {keys} in [{table}]")
    except BaseException:
        # SystemExit from fail() inside validate() must also drop the temp
        # file — the original is the only thing left behind on a rejection.
        temp_path.unlink(missing_ok=True)
        raise


if __name__ == "__main__":
    main(sys.argv)

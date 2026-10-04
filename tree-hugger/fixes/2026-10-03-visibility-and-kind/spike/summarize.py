#!/usr/bin/env python3
"""Summarize `hug --json symbols` output: one row per symbol, v1 and v2 side by side.

Usage: summarize.py raw/<lang>.json [...]

Columns: line:col, name, v1 kind, v1 container (kind name), v1 signature
present, v1 signature visibility / is_static, v2 kind, v2 visibility level,
v2 modifiers, v2 module_path. Type-metadata fields are listed under their
owning type with their visibility and static flag.
"""

import json
import sys


def v1_row(symbol):
    signature = symbol.get("signature")
    container = symbol.get("container_name") or ""
    if symbol.get("container_kind"):
        container = f"{symbol['container_kind']} {container}"
    return {
        "kind": symbol["kind"],
        "container": container,
        "sig": "sig" if signature is not None else "-",
        "sig_vis": (signature or {}).get("visibility") or "",
        "sig_static": "static" if (signature or {}).get("is_static") else "",
        "fields": (symbol.get("type_metadata") or {}).get("fields") or [],
    }


def v2_rows(index):
    rows = {}
    for record in index["symbols"]:
        span = record["source"]["declaration_span"]["start"]
        key = (record["identity"]["name"], span["line"], span["column"])
        modifiers = ",".join(m.get("type", "?") for m in record.get("modifiers") or [])
        rows[key] = {
            "kind": record["kind"],
            "level": record["visibility"]["level"],
            "mods": modifiers,
            "module_path": record["identity"].get("module_path") or "",
        }
    return rows


def summarize(path):
    document = json.load(open(path))
    file_entry = document["files"][0]
    v2 = v2_rows(document["symbol_indexes"][0])
    seen = set()
    print(f"## {path}  (schema {document.get('schema_version')})")
    print("line:col | name | v1 kind | v1 container | sig | sig.vis | sig.static || v2 kind | v2 level | v2 mods | v2 module_path")
    for symbol in sorted(file_entry["symbols"], key=lambda s: (s["range"]["start_line"], s["range"]["start_column"])):
        key = (symbol["name"], symbol["range"]["start_line"], symbol["range"]["start_column"])
        seen.add(key)
        left = v1_row(symbol)
        right = v2.get(key, {"kind": "?", "level": "?", "mods": "?", "module_path": "?"})
        print(
            f"{key[1]}:{key[2]} | {key[0]} | {left['kind']} | {left['container']} | {left['sig']} | "
            f"{left['sig_vis']} | {left['sig_static']} || {right['kind']} | {right['level']} | "
            f"{right['mods']} | {right['module_path']}"
        )
        for field in left["fields"]:
            print(
                f"    field {field.get('name')}: vis={field.get('visibility') or ''} "
                f"static={bool(field.get('is_static'))} type={field.get('type_annotation') or ''}"
            )
    for key, right in sorted(v2.items(), key=lambda item: (item[0][1], item[0][2])):
        if key not in seen:
            print(f"{key[1]}:{key[2]} | {key[0]} | (v2 only) || {right['kind']} | {right['level']} | {right['mods']} | {right['module_path']}")
    print()


if __name__ == "__main__":
    for argument in sys.argv[1:]:
        summarize(argument)

#!/usr/bin/env python3
"""Spike S2 scanner: crate-global and consolidation-sensitive constructs.

Evidence tool for Phase 1. Its detector list is the seed for Phase 2's
`consolidation.py check-attributes`; keep the two in step.

Usage: s2-scan.py [--json] (run from the repository root)
"""

from __future__ import annotations

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

PACKAGE_TEST_DIRS = {
    "claudine-cli": "claudine/cli/tests",
    "darkmatter": "darkmatter/lib/tests",
    "darkmatter-cli": "darkmatter/cli/tests",
    "biscuit-terminal": "biscuit-terminal/lib/tests",
}

# Attributes that rustc honors only at a crate root; inside a module they
# degrade to an `unused_attributes` warning and the setting is dropped (S1 §7).
CRATE_ROOT_ONLY = (
    "recursion_limit", "type_length_limit", "no_std", "no_main", "no_implicit_prelude",
    "feature", "windows_subsystem", "crate_type", "crate_name", "test_runner",
    "reexport_test_harness_main", "no_builtins", "compiler_builtins", "moved_panic_handler",
)

DETECTORS: dict[str, re.Pattern[str]] = {
    # Crate-global at link or crate-namespace level.
    "macro_export": re.compile(r"#\[\s*macro_export"),
    "global_allocator": re.compile(r"#\[\s*global_allocator"),
    "no_mangle": re.compile(r"#\[\s*(unsafe\s*\(\s*)?no_mangle"),
    "export_name": re.compile(r"#\[\s*(unsafe\s*\(\s*)?export_name"),
    "link_section": re.compile(r"#\[\s*(unsafe\s*\(\s*)?link_section"),
    "used": re.compile(r"#\[\s*used\b"),
    "link_attr": re.compile(r"#\[\s*link\s*\("),
    "ctor": re.compile(r"#\[\s*(ctor|dtor|ctor::ctor|dtor::dtor)\b"),
    "panic_handler": re.compile(r"#\[\s*panic_handler"),
    "fn_main": re.compile(r"^[ \t]*(pub[ \t]+)?fn[ \t]+main[ \t]*\(", re.M),
    # Consolidation-sensitive (module resolution, paths, identities).
    "macro_use": re.compile(r"#\[\s*macro_use"),
    "macro_rules": re.compile(r"^[ \t]*macro_rules![ \t]*\w+", re.M),
    "mod_common": re.compile(r"^[ \t]*(pub(\([a-z]+\))?[ \t]+)?mod[ \t]+common[ \t]*;", re.M),
    # A file-backed `mod x;` without `#[path]` resolves beside a crate root but
    # under `<file-stem>/` from a non-root module file (S1 §9), so every one
    # needs a path repair when its file becomes a module.
    "mod_decl_other": re.compile(r"^[ \t]*(pub(\([a-z]+\))?[ \t]+)?mod[ \t]+(?!common\b)\w+[ \t]*;", re.M),
    "path_attr": re.compile(r"#\[\s*path\s*="),
    "include_macro": re.compile(r"\binclude_(str|bytes)?!\s*\("),
    "file_macro": re.compile(r"\bfile!\s*\(\s*\)"),
    "module_path_macro": re.compile(r"\bmodule_path!\s*\(\s*\)"),
    "crate_name_env": re.compile(r"CARGO_CRATE_NAME"),
    "current_exe": re.compile(r"current_exe\s*\("),
    "exact_arg": re.compile(r"\"--exact\""),
    "insta": re.compile(r"\binsta::|assert_(snapshot|debug_snapshot|yaml_snapshot|json_snapshot|display_snapshot)!"),
    "extern_c_fn": re.compile(r"extern\s+\"C\"\s+fn\s+\w+"),
}

INNER_ATTR = re.compile(r"^\s*#!\[\s*([A-Za-z_][A-Za-z0-9_:]*)(.*)$")


def scan_file(path: Path) -> dict:
    text = path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    inner = []
    for number, line in enumerate(lines, 1):
        match = INNER_ATTR.match(line)
        if match:
            inner.append({"line": number, "name": match.group(1), "text": line.strip()})
    hits = {}
    for name, pattern in DETECTORS.items():
        found = [text.count("\n", 0, m.start()) + 1 for m in pattern.finditer(text)]
        if found:
            hits[name] = found
    return {"inner_attributes": inner, "hits": hits}


def classify_inner(name: str) -> str:
    if name in CRATE_ROOT_ONLY:
        return "crate-root-only"
    if name in ("cfg",):
        return "platform-or-feature-condition"
    if name == "cfg_attr":
        return "conditional-attribute"
    if name in ("allow", "warn", "deny", "expect", "forbid"):
        return "lint-level"
    if name == "doc":
        return "doc"
    return "other"


def main() -> int:
    repo = Path.cwd()
    report: dict[str, dict] = {}
    for package, rel in PACKAGE_TEST_DIRS.items():
        root = repo / rel
        files = {}
        for path in sorted(root.rglob("*.rs")):
            files[str(path.relative_to(repo))] = scan_file(path)
        report[package] = files

    if "--json" in sys.argv:
        json.dump(report, sys.stdout, indent=2, sort_keys=True)
        print()
        return 0

    for package, files in report.items():
        print(f"## {package} ({len(files)} .rs files under tests/)")
        inner_counts: dict[str, int] = defaultdict(int)
        inner_files: dict[str, list[str]] = defaultdict(list)
        for file, data in files.items():
            for attr in data["inner_attributes"]:
                key = f"{classify_inner(attr['name'])}: {attr['text']}"
                inner_counts[key] += 1
                inner_files[key].append(f"{file}:{attr['line']}")
        print("### inner attributes")
        for key in sorted(inner_counts, key=lambda k: (-inner_counts[k], k)):
            sample = inner_files[key] if inner_counts[key] <= 6 else inner_files[key][:3] + ["…"]
            print(f"- {inner_counts[key]:4d}  {key}  [{', '.join(sample)}]")
        print("### detector hits")
        for detector in DETECTORS:
            where = [f"{f}:{','.join(map(str, d['hits'][detector]))}" for f, d in files.items() if detector in d["hits"]]
            if where:
                sample = where if len(where) <= 8 else where[:6] + [f"… (+{len(where) - 6} files)"]
                print(f"- {detector}: {len(where)} files  [{'; '.join(sample)}]")
        print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

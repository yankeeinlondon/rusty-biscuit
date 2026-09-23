#!/usr/bin/env python3
"""Spike S2 (wave 2) scanner: crate-global, path, and identity hazards.

Read-only evidence tool. It imports the detector tables and parsers from
`scripts/ci/consolidation.py` (the first feature's S2 list, now used by
`check-attributes`) and applies them to every `.rs` file under the ten
wave-2 packages' `tests/` trees, plus a few wave-2 additions (crate-root
item names, file-relative fixture strings, self-referencing test paths).

Usage: s2-scan.py [--json] (run from the repository root)
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
spec = importlib.util.spec_from_file_location("consolidation", REPO / "scripts/ci/consolidation.py")
C = importlib.util.module_from_spec(spec)
sys.modules["consolidation"] = C
spec.loader.exec_module(C)

PACKAGES = {
    "tree-hugger": "tree-hugger/lib",
    "claudine": "claudine/lib",
    "sniff": "sniff/lib",
    "biscuit-file": "biscuit-file/lib",
    "schematic-gen": "schematic/gen",
    "biscuit-terminal-cli": "biscuit-terminal/cli",
    "claudine-gen": "claudine/gen",
    "dmls": "darkmatter/dmls",
    "sniff-cli": "sniff/cli",
    "biscuit-tui-cli": "biscuit-tui/cli",
}

EXTRA_DETECTORS = {
    "link_attr": re.compile(r"#\[\s*link\s*\("),
    "macro_use": re.compile(r"#\[\s*macro_use"),
    "macro_rules": re.compile(r"^[ \t]*macro_rules![ \t]*\w+", re.M),
    "fn_main": re.compile(r"^[ \t]*(pub[ \t]+)?fn[ \t]+main[ \t]*\(", re.M),
    "manifest_dir": re.compile(r"CARGO_MANIFEST_DIR|manifest_dir!\s*\("),
    "insta_assert": C.SNAPSHOT_ASSERT,
    "insta_settings": re.compile(C.INSTA_PATH_SETTINGS.pattern + r"|with_settings!"),
    "exact_any": re.compile(r"--exact"),
    "list_arg": C.LIST_ARG,
    "current_exe": C.CURRENT_EXE,
    "nextest_env": re.compile(r"NEXTEST_TEST_NAME|NEXTEST_BINARY_ID|CARGO_BIN_NAME|thread::current\(\)\.name"),
    "non_leading_inner_attr": re.compile(r"^[ \t]*#!\[", re.M),
}
# A string literal naming a test source path (`tests/foo.rs`, `tests/common/…`).
TEST_PATH_LITERAL = re.compile(r"\"[^\"\n]*\btests/[A-Za-z0-9_./-]*\"")
# A string literal starting with `../` or `./` (file-relative only inside include_*!/#[path]).
RELATIVE_LITERAL = re.compile(r"\"\.\.?/[^\"\n]*\"")
TOP_ITEM = re.compile(
    r"^(?:pub(?:\([a-z]+\))?\s+)?(?:async\s+|unsafe\s+|const\s+)*"
    r"(fn|struct|enum|const|static|type|trait|union|mod)\s+([A-Za-z_][A-Za-z0-9_]*)"
    r"|^macro_rules!\s*([A-Za-z_][A-Za-z0-9_]*)"
    r"|^(?:pub(?:\([a-z]+\))?\s+)?use\s+[^;]*?\bas\s+([A-Za-z_][A-Za-z0-9_]*)\s*;",
    re.M,
)


def top_level_items(text: str) -> list[tuple[str, str, int]]:
    """(kind, name, line) for items at column 0 outside strings/comments (heuristic)."""
    stripped = C._strip_comments(text)
    items = []
    for match in TOP_ITEM.finditer(stripped):
        if match.group(2):
            kind, name = match.group(1), match.group(2)
        elif match.group(3):
            kind, name = "macro_rules", match.group(3)
        else:
            kind, name = "use-as", match.group(4)
        items.append((kind, name, C._line_of(stripped, match.start())))
    return items


def scan_file(path: Path, rel_to_tests: str) -> dict:
    text = path.read_text(encoding="utf-8", errors="replace")
    leading = C.leading_inner_attributes(text)
    hits = {}
    for table in (C.CRATE_GLOBAL_DETECTORS, C.IDENTITY_DETECTORS, C.PATH_DETECTORS, EXTRA_DETECTORS):
        hits.update(C._detector_hits(text, table))
    stripped = C._strip_comments(text)
    if C.CRATE_PATH.search(stripped):
        hits["crate_path"] = [C._line_of(stripped, m.start()) for m in C.CRATE_PATH.finditer(stripped)]
    if C.CURRENT_EXE.search(text) and C.LIST_ARG.search(text):
        hits["current_exe_list"] = hits["current_exe"]
    literals = {}
    for name, pattern in (("test_path_literal", TEST_PATH_LITERAL), ("relative_literal", RELATIVE_LITERAL)):
        found = [(C._line_of(stripped, m.start()), m.group(0)) for m in pattern.finditer(stripped)]
        if found:
            literals[name] = found
    mods = C.module_declarations(text)
    return {
        "leading_inner": leading,
        "hits": hits,
        "literals": literals,
        "mod_decls": mods,
        "top_items": top_level_items(text) if "/" not in rel_to_tests else [],
    }


def classify(attribute: str) -> str:
    name = C._attribute_name(attribute)
    if name in C.CRATE_ROOT_ONLY:
        return "CRATE-ROOT-ONLY"
    if name == "cfg":
        return "cfg"
    if name in C.LINT_ATTRIBUTES:
        return "lint"
    return name


def scan_package(manifest_dir: str) -> dict:
    tests = REPO / manifest_dir / "tests"
    files = {}
    for path in sorted(tests.rglob("*.rs")):
        rel = path.relative_to(tests).as_posix()
        if rel.startswith("fixtures/") or "/fixtures/" in rel:
            continue  # fixture data, not compiled test source
        files[rel] = scan_file(path, rel)
    snaps = sorted(p.relative_to(REPO).as_posix() for p in tests.rglob("*.snap"))
    return {"files": files, "snapshots": snaps}


def main() -> int:
    report = {pkg: scan_package(d) for pkg, d in PACKAGES.items()}
    if "--json" in sys.argv:
        json.dump(report, sys.stdout, indent=2, sort_keys=True, default=list)
        print()
        return 0
    for pkg, data in report.items():
        files = data["files"]
        roots = [f for f in files if "/" not in f]
        print(f"## {pkg} ({PACKAGES[pkg]}/tests: {len(roots)} target files, {len(files) - len(roots)} helper files, {len(data['snapshots'])} .snap)")
        print("### leading inner attributes")
        for f, d in files.items():
            for a in d["leading_inner"]:
                print(f"- [{classify(a)}] {f}: #![{a}]")
        print("### detector hits")
        for f, d in files.items():
            for det, lines in sorted(d["hits"].items()):
                print(f"- {det}: {f}:{','.join(map(str, lines[:12]))}{' …' if len(lines) > 12 else ''} ({len(lines)})")
        print("### mod declarations")
        for f, d in files.items():
            for name, attrs in d["mod_decls"].items():
                print(f"- {f}: {''.join(f'#[{a}] ' for a in attrs)}mod {name};")
        print("### string literals naming tests/ paths or ../ paths")
        for f, d in files.items():
            for kind, found in d["literals"].items():
                for line, lit in found:
                    print(f"- {kind}: {f}:{line} {lit}")
        print("### crate-root item names appearing in more than one target file")
        owners = defaultdict(list)
        for f in roots:
            for kind, name, line in files[f]["top_items"]:
                owners[(kind, name)].append(f"{f}:{line}")
        dups = {k: v for k, v in owners.items() if len({x.split(':')[0] for x in v}) > 1}
        for (kind, name), where in sorted(dups.items(), key=lambda kv: (-len(kv[1]), kv[0])):
            print(f"- {kind} {name}: {len(where)} files [{', '.join(where[:6])}{' …' if len(where) > 6 else ''}]")
        if data["snapshots"]:
            print("### snapshots")
            for s in data["snapshots"]:
                print(f"- {s}")
        print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

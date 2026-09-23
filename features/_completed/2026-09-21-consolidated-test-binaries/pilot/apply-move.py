#!/usr/bin/env python3
"""Apply the mechanical half of one package's consolidation from its manifest.

Reads a `consolidation.py plan` manifest and, for every module row:

- moves `old_path` to `new_path` (a nested crate root moves with its whole
  directory, and its `main.rs` becomes `mod.rs` so every child still resolves);
- deletes the leading `#![cfg(...)]` the manifest records in `inner_cfg`
  (the same condition goes onto the module declaration, acceptance 4);
- rewrites the file's own `mod common;` to `use crate::common;`, keeping any
  attribute or doc line above it (spec §3: one `common` per crate root);
- prefixes `../` to every relative `include_str!`/`include_bytes!` path,
  because the file moved one directory deeper.

It then writes one `main.rs` per consolidated target: a shared
`#[path = "../common/mod.rs"] mod common;` when the package has a
`tests/common/`, and one declaration per module, carrying the module's former
crate-level `cfg`.

Everything else — helper directories, `#[path]` helper modules, guards that
key on paths, `Cargo.toml`, snapshots — is a reviewed hand edit, recorded in
the implementation log. This script is the record of how the mechanical part
was produced, not a general tool: `consolidation.py compare`,
`check-attributes`, and `check-snapshots` are what prove the result.

Usage (from the repository root, on a clean tree; the second argument names
the `l1` module holding the package's layout gate, `test_placement.rs` for the
pilot):

    python3 features/2026-09-21-consolidated-test-binaries/pilot/apply-move.py \
        features/2026-09-21-consolidated-test-binaries/claudine-cli-migration.json \
        test_placement.rs
"""

from __future__ import annotations

import json
import re
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
MOD_COMMON = re.compile(r"^([ \t]*)mod common;[ \t]*$", re.M)
INCLUDE = re.compile(r'(\binclude_(?:str|bytes)!\s*\(\s*")([^"]+)(")')

TIER_NAMES = {
    "L1": "Level 1",
    "L2": "Level 2",
    "L3": "Level 3",
    "browser": "browser",
    "real": "real-provider",
}


def strip_inner_cfg(text: str, conditions: list[str], path: str) -> str:
    for condition in conditions:
        pattern = re.compile(rf"^#!\[cfg\({re.escape(condition)}\)\][ \t]*\n(\n)?", re.M)
        # The attribute and the blank line under it go; the blank line that
        # separated it from the doc block above stays as the separator.
        text, count = pattern.subn("", text, count=1)
        if count != 1:
            raise SystemExit(f"{path}: expected one #![cfg({condition})]")
    return text


def repair_includes(text: str) -> str:
    return INCLUDE.sub(lambda m: m.group(1) + ("../" + m.group(2) if not m.group(2).startswith("/") else m.group(2)) + m.group(3), text)


def root_source(target: dict, modules: dict[str, dict], package: str, guard: str, common: bool) -> str:
    tier = TIER_NAMES.get(target["tier"], target["tier"])
    if target["required_features"]:
        tier += " (" + ", ".join(f"`{f}`" for f in target["required_features"]) + ")"
    lines = [
        f"//! {tier} integration tests for `{package}`, one test binary per",
        "//! execution contract (`2026-09-21-consolidated-test-binaries`).",
        "//!",
        "//! Each module was its own test target before the consolidation and keeps",
        "//! that target's name, which is the first segment of every test path here.",
        "//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is",
        f"//! not declared below never compiles; `{guard}` rejects one.",
        "",
    ]
    if common:
        lines += ['#[path = "../common/mod.rs"]', "mod common;", ""]
    for name in target["modules"]:
        for condition in modules[name]["inner_cfg"]:
            lines.append(f"#[cfg({condition})]")
        lines.append(f"mod {name};")
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    manifest = json.loads(Path(argv[1]).read_text(encoding="utf-8"))
    guard = argv[2] if len(argv) > 2 else "test_placement.rs"
    common = (REPO / manifest["crate_dir"] / "tests" / "common" / "mod.rs").is_file()
    modules = {row["module"]: row for row in manifest["modules"]}
    for row in manifest["modules"]:
        old, new = REPO / row["old_path"], REPO / row["new_path"]
        new.parent.mkdir(parents=True, exist_ok=True)
        # A plain move, not `git mv`: the index is left for the author to stage.
        if row["nested_root"]:
            new.parent.rmdir()
            shutil.move(old.parent, new.parent)
            (new.parent / old.name).rename(new)
        else:
            shutil.move(old, new)
        text = new.read_text(encoding="utf-8")
        text = strip_inner_cfg(text, row["inner_cfg"], row["new_path"])
        text = MOD_COMMON.sub(r"\1use crate::common;", text)
        text = repair_includes(text)
        new.write_text(text, encoding="utf-8")
    for target in manifest["targets"]:
        root = REPO / target["path"]
        root.write_text(root_source(target, modules, manifest["package"], guard, common), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

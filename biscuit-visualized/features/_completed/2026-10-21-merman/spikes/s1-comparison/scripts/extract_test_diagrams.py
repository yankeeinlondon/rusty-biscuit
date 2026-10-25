#!/usr/bin/env python3
"""Extract the Mermaid sources embedded in the repository's Rust tests (R-S1 b, AC-S2).

Scans every Rust string literal (plain, raw, and fenced ```mermaid blocks inside
literals) in the test files listed below, keeps those whose first meaningful
line is a Mermaid header, deduplicates on whitespace-normalized text, and writes
one `.mmd` per diagram into `corpus/tests/` plus `corpus/tests/manifest.json`
recording every place each source appeared.

Usage: python3 -I scripts/extract_test_diagrams.py <repo-root> <spike-dir>
"""

import hashlib
import json
import re
import sys
from pathlib import Path

TEST_FILES = [
    "biscuit-visualized/src/src/tests/mermaid_tests.rs",
    "biscuit-visualized/src/src/tests/gitgraph_tests.rs",
    "biscuit-visualized/src/src/tests/cache_tests.rs",
    "biscuit-visualized/src/src/components/git_graph/tests.rs",
    "biscuit-visualized/src/tests/l1/mermaid_parity.rs",
    "biscuit-visualized/src/tests/l1/layout_matrix.rs",
    "biscuit-visualized/src/tests/l1/render_tree_diagram_hook.rs",
    "biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs",
    "biscuit-terminal/lib/tests/l1/render_tree_code_context.rs",
    "biscuit-terminal/cli/tests/l1/integration_test.rs",
    "biscuit-terminal/cli/tests/level2/diagrams.rs",
    "darkmatter/lib/tests/l1/render_invariants.rs",
    "darkmatter/lib/tests/l1/reference_integration.rs",
    "darkmatter/lib/tests/l1/style_features_baseline.rs",
    "darkmatter/lib/tests/l1/style_features_phase5.rs",
    "darkmatter/lib/tests/browser/browser_render.rs",
    "darkmatter/cli/tests/l1/validate_refs.rs",
    "darkmatter/dmls/tests/l1/lsp_session.rs",
    "renderable/tests/style_features_baseline.rs",
    "renderable/tests/html_page_feature_injection.rs",
    "worktree/cli/src/commands/git_graph/tests.rs",
]

HEADERS = [
    "flowchart", "graph", "sequenceDiagram", "classDiagram", "stateDiagram",
    "erDiagram", "pie", "xychart", "quadrantChart", "sankey", "gantt",
    "timeline", "journey", "kanban", "C4Context", "C4Container", "C4Component",
    "C4Dynamic", "C4Deployment", "block", "architecture", "requirementDiagram",
    "mindmap", "gitGraph", "zenuml", "packet", "radar", "treemap", "info",
    "venn", "ishikawa", "wardley", "treeView",
]

# Family ids match the Merman fixture directory names used for the gallery sections.
HEADER_FAMILY = {
    "flowchart": "flowchart", "graph": "flowchart", "sequenceDiagram": "sequence",
    "classDiagram": "class", "stateDiagram": "state", "erDiagram": "er", "pie": "pie",
    "xychart": "xychart", "quadrantChart": "quadrantchart", "sankey": "sankey",
    "gantt": "gantt", "timeline": "timeline", "journey": "journey", "kanban": "kanban",
    "C4Context": "c4", "C4Container": "c4", "C4Component": "c4", "C4Dynamic": "c4",
    "C4Deployment": "c4", "block": "block", "architecture": "architecture",
    "requirementDiagram": "requirement", "mindmap": "mindmap", "gitGraph": "gitgraph",
    "zenuml": "zenuml", "packet": "packet", "radar": "radar", "treemap": "treemap",
    "info": "info", "venn": "venn", "ishikawa": "ishikawa", "wardley": "wardley",
    "treeView": "treeView",
}

LITERAL = re.compile(
    r'(?P<raw>\br(?P<hashes>#*)"(?P<rawbody>.*?)"(?P=hashes))'
    r'|(?P<plain>(?<![A-Za-z0-9_\'])"(?P<body>(?:[^"\\]|\\.)*)")',
    re.S,
)
# `gitgraph_tests.rs` builds sources with `gitgraph(&["commit", ...])`, which
# prefixes `gitGraph` and indents each line; rebuild those the same way.
HELPER = re.compile(r"\bgitgraph\(&\[(?P<items>.*?)\]\)", re.S)
TEMPLATE = re.compile(r"\{[A-Za-z_]*\}")
FENCE = re.compile(r"```mermaid[^\n]*\n(.*?)```", re.S)


def unescape(body: str) -> str:
    body = re.sub(r"\\\n\s*", "", body)  # Rust line continuation
    out, i = [], 0
    while i < len(body):
        c = body[i]
        if c == "\\" and i + 1 < len(body):
            n = body[i + 1]
            mapping = {"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\", "'": "'", "0": "\0"}
            if n in mapping:
                out.append(mapping[n]); i += 2; continue
            if n == "u":
                m = re.match(r"\\u\{([0-9a-fA-F]+)\}", body[i:])
                if m:
                    out.append(chr(int(m.group(1), 16))); i += len(m.group(0)); continue
        out.append(c); i += 1
    return "".join(out)


def header_of(text: str):
    lines = text.strip().splitlines()
    i = 0
    if lines and lines[0].strip() == "---":  # frontmatter
        j = 1
        while j < len(lines) and lines[j].strip() != "---":
            j += 1
        i = j + 1
    while i < len(lines):
        line = lines[i].strip()
        if not line or line.startswith("%%"):
            i += 1
            continue
        for h in HEADERS:
            if re.match(rf"{re.escape(h)}(-v2|-beta)?(\s|;|$)", line):
                return h
        return None
    return None


def tidy(text: str) -> str:
    lines = text.strip("\n").splitlines()
    return "\n".join([lines[0].lstrip()] + lines[1:]).rstrip() + "\n"


def main() -> None:
    repo, spike = Path(sys.argv[1]), Path(sys.argv[2])
    out_dir = spike / "corpus" / "tests"
    out_dir.mkdir(parents=True, exist_ok=True)
    found: dict[str, dict] = {}
    for rel in TEST_FILES:
        path = repo / rel
        if not path.exists():
            continue
        source = path.read_text()
        literals = []
        for m in HELPER.finditer(source):
            items = m.group("items")
            if "format!" in items:
                continue  # interpolated constants; not reproducible statically
            lines = [unescape(x) for x in re.findall(r'"((?:[^"\\]|\\.)*)"', items)]
            if lines:
                text = "gitGraph\n" + "\n".join(f"    {l}" for l in lines)
                literals.append((text, source.count("\n", 0, m.start()) + 1))
        for m in LITERAL.finditer(source):
            body = m.group("rawbody") if m.group("raw") else unescape(m.group("body"))
            literals.append((body, source.count("\n", 0, m.start()) + 1))
        for body, line in literals:
            candidates = FENCE.findall(body) or [body]
            for cand in candidates:
                header = header_of(cand)
                # Skip format! templates (`{body}`) and bare headers used as substrings.
                if header is None or TEMPLATE.search(cand) or len(cand.strip().splitlines()) < 2 and ";" not in cand:
                    continue
                text = tidy(cand)
                key = re.sub(r"\s+", " ", text).strip()
                if len(key) < 12:
                    continue
                entry = found.setdefault(key, {"text": text, "header": header, "occurrences": []})
                entry["occurrences"].append(f"{rel}:{line}")
    manifest = []
    for key, entry in found.items():
        family = HEADER_FAMILY[entry["header"]]
        digest = hashlib.sha1(key.encode()).hexdigest()[:8]
        name = f"test_{family}_{digest}"
        (out_dir / f"{name}.mmd").write_text(entry["text"])
        manifest.append({"name": name, "family": family, "occurrences": entry["occurrences"]})
    manifest.sort(key=lambda e: (e["family"], e["name"]))
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"extracted {len(manifest)} unique Mermaid sources from {len(TEST_FILES)} files")


if __name__ == "__main__":
    main()

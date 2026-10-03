#!/usr/bin/env python3
"""THROWAWAY SPIKE: rough call-site counts of expression functions in Markdown.

Counts `name(` inside `{{ ... }}` spans and on lines with `when=` or `$(`.
Read-only. Usage: python3 callsites.py <baseline.json> <out.json> <root>...
"""
import json
import os
import re
import sys

baseline, out_path, roots = sys.argv[1], sys.argv[2], sys.argv[3:]
names = sorted(json.load(open(baseline))["signatures"].keys(), key=len, reverse=True)
pattern = re.compile(r"\b(" + "|".join(map(re.escape, names)) + r")\s*\(", re.IGNORECASE)
span = re.compile(r"\{\{.*?\}\}", re.S)
SKIP = {"target", "node_modules", ".gitnexus", "research"}

counts = {root: {} for root in roots}
files_with = {root: {} for root in roots}
for root in roots:
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".git")]
        for f in filenames:
            if not f.endswith(".md"):
                continue
            path = os.path.join(dirpath, f)
            try:
                text = open(path, encoding="utf-8", errors="ignore").read()
            except OSError:
                continue
            chunks = span.findall(text) + [l for l in text.splitlines() if "when=" in l or "$(" in l]
            seen = set()
            for chunk in chunks:
                for m in pattern.finditer(chunk):
                    n = m.group(1).lower()
                    counts[root][n] = counts[root].get(n, 0) + 1
                    seen.add(n)
            for n in seen:
                files_with[root][n] = files_with[root].get(n, 0) + 1

json.dump({"counts": counts, "files": files_with}, open(out_path, "w"), indent=2)
for root in roots:
    top = sorted(counts[root].items(), key=lambda kv: -kv[1])
    print(root, sum(counts[root].values()), top[:40])

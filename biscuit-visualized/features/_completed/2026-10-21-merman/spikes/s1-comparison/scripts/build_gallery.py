#!/usr/bin/env python3
"""Build the self-contained comparison gallery (D2, R-S3) and `gallery/scorecard.json`.

Reads the renderer's `results-<os>.json` plus the PNGs it wrote, the corpus
(Mermaid.js references, mmdc results), optional build-weight and cross-OS
JSON, and `scripts/observations.json` (reviewer notes per family). PNGs are
recompressed losslessly with ImageMagick before embedding (tiny-skia writes them
barely compressed); pixels are unchanged.

Usage:
  python3 -I scripts/build_gallery.py --spike . --results <results.json> \
      --theme dark --out gallery/index.html [--scorecard gallery/scorecard.json] \
      [--build-weight bw.json] [--cross-os cross-os.json] [--other-results linux.json] \
      [--png-cache <dir>]
"""

import argparse
import base64
import hashlib
import html
import json
import re
import statistics
import subprocess
import urllib.parse
from pathlib import Path

FAMILY_ORDER = [
    "flowchart", "sequence", "class", "state", "er", "gitgraph", "pie", "xychart",
    "quadrantchart", "gantt", "timeline", "journey", "mindmap", "kanban", "sankey",
    "c4", "block", "architecture", "requirement", "packet", "radar", "treemap", "zenuml",
    "info", "venn", "ishikawa", "wardley", "treeView", "eventmodeling", "cynefin",
    "railroad", "railroadAbnf", "railroadEbnf", "railroadPeg", "swimlane", "usecase", "agentflow",
]
FAMILY_LABEL = {
    "flowchart": "Flowchart", "sequence": "Sequence", "class": "Class", "state": "State",
    "er": "Entity relationship", "gitgraph": "gitGraph", "pie": "Pie", "xychart": "XY chart",
    "quadrantchart": "Quadrant chart", "gantt": "Gantt", "timeline": "Timeline",
    "journey": "User journey", "mindmap": "Mindmap", "kanban": "Kanban", "sankey": "Sankey",
    "c4": "C4", "block": "Block", "architecture": "Architecture", "requirement": "Requirement",
    "packet": "Packet", "radar": "Radar", "treemap": "Treemap", "zenuml": "ZenUML",
    "info": "Info", "venn": "Venn", "ishikawa": "Ishikawa", "wardley": "Wardley map",
    "treeView": "Tree view", "eventmodeling": "Event modeling", "cynefin": "Cynefin",
    "railroad": "Railroad", "railroadAbnf": "Railroad (ABNF)", "railroadEbnf": "Railroad (EBNF)",
    "railroadPeg": "Railroad (PEG)", "swimlane": "Swimlane", "usecase": "Use case",
    "agentflow": "Agent flow",
}
ORIGIN_ORDER = {"merman-fixture": 0, "gitgraph-case": 1, "repo-test": 2}
RENDERERS = [
    ("current", "Current (biscuit-visualized + mermaid-rs-renderer)"),
    ("merman", "Merman"),
    ("merman-real", "Merman (real fonts)"),
]
REV_SHORT = "482fcde"
# Published copies of the three pages: file name -> (link label, <title>, URL).
VARIANTS = {
    "index.html": ("macOS, dark", "Merman Bake-off",
                   "https://claude.ai/code/artifact/ff37a2ed-dd43-4b0a-af59-7a9f1055a0b3"),
    "index-light.html": ("macOS, light", "Merman Bake-off Light",
                         "https://claude.ai/code/artifact/5a9a7bc0-7850-4602-9c84-501468112bed"),
    "index-linux.html": ("Linux, dark", "Merman Bake-off Linux",
                         "https://claude.ai/code/artifact/6e0e1be7-588b-4a0b-9d12-d0871ac3ca72"),
}


def scrub_home(text: str) -> str:
    """Error messages carry absolute paths; keep the user's home out of shared pages."""
    return text.replace(str(Path.home()), "~").replace("/home/build", "~")


def esc(text) -> str:
    return html.escape(str(text), quote=True)


class PngStore:
    """Lossless recompression with a content-addressed cache."""

    def __init__(self, cache: Path):
        self.cache = cache
        cache.mkdir(parents=True, exist_ok=True)

    def data_uri(self, path: str) -> str:
        raw = Path(path).read_bytes()
        key = hashlib.sha1(raw).hexdigest()
        out = self.cache / f"{key}.png"
        if not out.exists():
            subprocess.run(
                ["magick", path, "-strip", "-define", "png:compression-level=9",
                 "-define", "png:compression-strategy=1", "-define", "png:compression-filter=5",
                 f"PNG32:{out}"],
                check=True,
            )
            if out.stat().st_size > len(raw):
                out.write_bytes(raw)
        return "data:image/png;base64," + base64.b64encode(out.read_bytes()).decode()


def svg_data_uri(path: Path) -> str:
    text = path.read_text()
    return "data:image/svg+xml;charset=utf-8," + urllib.parse.quote(text, safe=" =:/;,()-._~@")


def fmt_ms(value):
    if value is None:
        return "n/a"
    return f"{value:.2f}" if value < 10 else f"{value:.1f}"


def classify(row, theme, mmdc_failures):
    """Per-input status for each renderer, plus whether the input is valid Mermaid."""
    inp = row["input"]
    invalid = inp["origin"] == "repo-test" and inp["name"] in mmdc_failures
    statuses = {}
    for key, _ in RENDERERS:
        res = row["results"].get(theme, {}).get(key)
        if res is None:
            statuses[key] = "absent"
        elif inp["group"] == "merman-only" and key == "current":
            statuses[key] = "unsupported-fallback" if res["ok"] else "unsupported"
        elif invalid:
            statuses[key] = "rejected-ok" if not res["ok"] else "accepted-invalid"
        else:
            statuses[key] = "rendered" if res["ok"] else "failed"
    return invalid, statuses


PILL = {
    "rendered": ("ok", "rendered"),
    "failed": ("fail", "failed"),
    "unsupported": ("unsup", "unsupported"),
    "unsupported-fallback": ("unsup", "unsupported: wrong diagram"),
    "rejected-ok": ("ok", "rejected, as Mermaid.js does"),
    "accepted-invalid": ("warn", "rendered input Mermaid.js rejects"),
    "absent": ("unsup", "not run"),
    "partial": ("warn", "partial"),
}


def pill(status, label=None):
    cls, text = PILL[status]
    return f'<span class="pill pill-{cls}">{esc(label or text)}</span>'


def family_summary(rows, theme, mmdc_failures):
    families = {}
    for row in rows:
        fam = row["input"]["family"]
        f = families.setdefault(fam, {
            "family": fam, "group": row["input"]["group"], "inputs": 0, "valid_inputs": 0,
            "ok": {k: 0 for k, _ in RENDERERS}, "medians": {k: [] for k, _ in RENDERERS},
            "p95s": {k: [] for k, _ in RENDERERS}, "io": [], "fix": [], "fallback": 0,
        })
        f["inputs"] += 1
        invalid, statuses = classify(row, theme, mmdc_failures)
        if row["input"]["group"] == "merman-only":
            f["group"] = "merman-only"
        if invalid:
            continue
        f["valid_inputs"] += 1
        both_ok = True
        for key, _ in RENDERERS:
            if statuses[key] == "rendered":
                f["ok"][key] += 1
            else:
                both_ok = False
            if statuses[key] == "unsupported-fallback":
                f["fallback"] += 1
        timing_res = row["results"].get("dark", {})
        for key, _ in RENDERERS:
            res = timing_res.get(key)
            if res and res.get("timing") and (both_ok or f["group"] == "merman-only"):
                if f["group"] == "merman-only" and key == "current":
                    continue
                f["medians"][key].append(res["timing"]["median_ms"])
                f["p95s"][key].append(res["timing"]["p95_ms"])
                if res.get("cache_io"):
                    f["io"].append(res["cache_io"]["median_ms"])
                if res.get("fix_timing"):
                    f["fix"].append(res["fix_timing"]["median_ms"])
    for f in families.values():
        f["status"] = {}
        for key, _ in RENDERERS:
            if f["group"] == "merman-only" and key == "current":
                f["status"][key] = "unsupported-fallback" if f["fallback"] else "unsupported"
            elif f["valid_inputs"] == 0:
                f["status"][key] = "absent"
            elif f["ok"][key] == f["valid_inputs"]:
                f["status"][key] = "rendered"
            elif f["ok"][key] == 0:
                f["status"][key] = "failed"
            else:
                f["status"][key] = "partial"
        f["median_ms"] = {k: (statistics.median(v) if v else None) for k, v in f["medians"].items()}
        f["p95_ms"] = {k: (max(v) if v else None) for k, v in f["p95s"].items()}
        c, m = f["median_ms"]["current"], f["median_ms"]["merman"]
        f["ratio"] = (m / c) if (c and m) else None
        f["cache_io_ms"] = statistics.median(f["io"]) if f["io"] else None
        net = (c - f["cache_io_ms"]) if (c and f["cache_io_ms"] is not None) else None
        f["current_net_ms"] = net
        f["ratio_net"] = (m / net) if (m and net and net > 0) else None
        f["fix_us"] = statistics.median(f["fix"]) * 1000 if f["fix"] else None
    return families


CSS = """
:root {
  --bg: #f6f8fa; --surface: #ffffff; --surface-2: #eef2f6; --border: #d5dce4;
  --text: #17202a; --muted: #5a6877; --accent: #1f5f8b; --accent-soft: #e3eef6;
  --focus: #1f5f8b;
  --ok: #1d6b3c; --ok-bg: #e2f3e8; --fail: #a3251c; --fail-bg: #fbe4e1;
  --warn: #8a5a00; --warn-bg: #fbf0d6; --unsup: #4f5b67; --unsup-bg: #e8ecf0;
  --render-dark: #16181d; --render-light: #ffffff; --render-ref: #ffffff;
  --code-bg: #f0f3f6;
  color-scheme: light;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #0f1419; --surface: #161c23; --surface-2: #1c242d; --border: #2b3540;
    --text: #e3e9ef; --muted: #93a1b0; --accent: #6fb3dd; --accent-soft: #19303f;
    --focus: #8cc6ea;
    --ok: #7fd3a0; --ok-bg: #173323; --fail: #f19a8f; --fail-bg: #3a1c19;
    --warn: #e9c46a; --warn-bg: #33290f; --unsup: #aab6c2; --unsup-bg: #252e37;
    --render-dark: #16181d; --render-light: #ffffff; --render-ref: #ffffff;
    --code-bg: #11171d;
    color-scheme: dark;
  }
}
:root[data-theme="dark"] {
  --bg: #0f1419; --surface: #161c23; --surface-2: #1c242d; --border: #2b3540;
  --text: #e3e9ef; --muted: #93a1b0; --accent: #6fb3dd; --accent-soft: #19303f;
  --focus: #8cc6ea;
  --ok: #7fd3a0; --ok-bg: #173323; --fail: #f19a8f; --fail-bg: #3a1c19;
  --warn: #e9c46a; --warn-bg: #33290f; --unsup: #aab6c2; --unsup-bg: #252e37;
  --render-dark: #16181d; --render-light: #ffffff; --render-ref: #ffffff;
  --code-bg: #11171d;
  color-scheme: dark;
}
* { box-sizing: border-box; }
body {
  margin: 0; background: var(--bg); color: var(--text);
  font: 15px/1.55 "IBM Plex Sans", system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  padding-left: max(16px, env(safe-area-inset-left, 0px));
  padding-right: max(16px, env(safe-area-inset-right, 0px));
  padding-block: 24px 64px;
}
main { max-width: 1480px; margin: 0 auto; }
h1, h2, h3 { text-wrap: balance; line-height: 1.25; }
h1 { font-size: 1.75rem; margin: 0 0 8px; }
h2 { font-size: 1.35rem; margin: 40px 0 8px; padding-top: 8px; border-top: 1px solid var(--border); }
h3 { font-size: 1.02rem; margin: 0; font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace; font-weight: 500; overflow-wrap: anywhere; }
p { max-width: 80ch; }
a { color: var(--accent); }
a:focus-visible, summary:focus-visible, .imglink:focus-visible {
  outline: 2px solid var(--focus); outline-offset: 2px; border-radius: 3px;
}
.lede { color: var(--muted); }
code, pre, .mono, td.num { font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace; }
.num, td.num { font-variant-numeric: tabular-nums; text-align: right; white-space: nowrap; }
.variants { display: flex; flex-wrap: wrap; gap: 8px 16px; margin: 12px 0 0; font-size: 0.9rem; }
.variants .here { font-weight: 600; }
nav.families {
  position: sticky; top: env(safe-area-inset-top, 0px); z-index: 5;
  background: var(--bg); border-bottom: 1px solid var(--border);
  padding: 8px 0; margin: 16px 0 0; display: flex; flex-wrap: wrap; gap: 4px 6px;
  font-size: 0.82rem;
}
nav.families a {
  text-decoration: none; padding: 2px 8px; border-radius: 999px;
  background: var(--surface-2); color: var(--text); border: 1px solid var(--border);
}
nav.families a:hover { background: var(--accent-soft); }
nav.families .sep { color: var(--muted); padding: 2px 4px; }
.table-wrap { overflow-x: auto; border: 1px solid var(--border); border-radius: 6px; background: var(--surface); margin-top: 12px; }
table { border-collapse: collapse; width: 100%; font-size: 0.86rem; }
th, td { padding: 6px 10px; border-bottom: 1px solid var(--border); text-align: left; vertical-align: top; }
th { background: var(--surface-2); font-weight: 600; white-space: nowrap; }
tr:last-child td { border-bottom: none; }
td.verdict { min-width: 9rem; }
td.note { min-width: 18rem; color: var(--muted); }
tfoot td { font-weight: 600; background: var(--surface-2); }
.pill { display: inline-block; padding: 1px 8px; border-radius: 999px; font-size: 0.78rem; font-weight: 600; white-space: nowrap; }
.pill-ok { color: var(--ok); background: var(--ok-bg); }
.pill-fail { color: var(--fail); background: var(--fail-bg); }
.pill-warn { color: var(--warn); background: var(--warn-bg); }
.pill-unsup { color: var(--unsup); background: var(--unsup-bg); }
section.family { scroll-margin-top: 64px; }
.family-note { color: var(--muted); font-size: 0.92rem; }
article.input { background: var(--surface); border: 1px solid var(--border); border-radius: 6px; padding: 14px; margin: 14px 0; }
.input-head { display: flex; flex-wrap: wrap; gap: 6px 12px; align-items: baseline; justify-content: space-between; }
.source-ref { color: var(--muted); font-size: 0.82rem; overflow-wrap: anywhere; margin: 4px 0 0; }
details { margin: 8px 0 0; }
summary { cursor: pointer; color: var(--accent); font-size: 0.86rem; }
pre { background: var(--code-bg); border: 1px solid var(--border); border-radius: 4px; padding: 10px; overflow-x: auto; font-size: 0.8rem; line-height: 1.45; margin: 6px 0 0; }
.panels { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; margin-top: 12px; }
@media (max-width: 1400px) { .panels { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
@media (max-width: 900px) { .panels { grid-template-columns: minmax(0, 1fr); } }
td.delta { white-space: nowrap; }
.better { color: var(--ok); font-weight: 600; }
.worse { color: var(--fail); font-weight: 600; }
figure { margin: 0; display: flex; flex-direction: column; min-width: 0; }
.panel-title { font-size: 0.8rem; font-weight: 600; margin: 0 0 6px; color: var(--muted); }
.canvas { border: 1px solid var(--border); border-radius: 4px; padding: 10px; display: flex; align-items: flex-start; justify-content: center; min-height: 80px; }
.canvas.dark { background: var(--render-dark); }
.canvas.light { background: var(--render-light); }
.canvas.ref { background: var(--render-ref); }
.canvas img { max-width: 100%; height: auto; display: block; }
.imglink { display: block; max-width: 100%; }
figcaption { font-size: 0.78rem; color: var(--muted); margin-top: 6px; font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
.failure { background: var(--fail-bg); color: var(--fail); border-radius: 4px; padding: 10px; font-size: 0.82rem; font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace; overflow-wrap: anywhere; white-space: pre-wrap; }
.neutral-box { background: var(--surface-2); color: var(--muted); border-radius: 4px; padding: 10px; font-size: 0.82rem; }
.notes { font-size: 0.92rem; }
.notes li { margin: 4px 0; max-width: 90ch; }
"""


def render_panel(row, theme, key, label, store, mmdc_failures):
    inp = row["input"]
    res = row["results"].get(theme, {}).get(key)
    surface = "dark" if theme == "dark" else "light"
    invalid, statuses = classify(row, theme, mmdc_failures)
    status = statuses[key]
    head = f'<p class="panel-title">{esc(label)} {pill(status)}</p>'
    if res is None:
        return f"<figure>{head}<div class='neutral-box'>Not rendered on this host.</div></figure>"
    if not res["ok"]:
        return f"<figure>{head}<div class='failure'>{esc(res.get('error') or 'failed')}</div></figure>"
    uri = store.data_uri(res["png"])
    width = max(1, round(res["svg_width"]))
    alt = f"{label} render of {inp['name']}"
    timing = res.get("timing")
    caption = []
    if timing:
        caption.append(f"median {fmt_ms(timing['median_ms'])} ms · p95 {fmt_ms(timing['p95_ms'])} ms")
        if res.get("cache_io"):
            caption.append(f"of which cache I/O ≈ {fmt_ms(res['cache_io']['median_ms'])} ms")
        if res.get("timing_warm"):
            caption.append(f"cold measurement cache; warm {fmt_ms(res['timing_warm']['median_ms'])} ms")
    defects = res.get("defects")
    if defects and defects["clipped"]:
        caption.append(f"{defects['clipped']} of {defects['labels']} labels clipped (worst {defects['worst_clip_px']:.1f}px)")
    caption.append(f"{res['png_width']}×{res['png_height']} px (SVG {res['svg_width']:.0f}×{res['svg_height']:.0f})")
    caption.append(f"raster {fmt_ms(res['raster_ms'])} ms")
    if res.get("has_foreign_object"):
        caption.append("contains foreignObject")
    if status == "unsupported-fallback":
        caption.insert(0, "mermaid-rs-renderer read this as a flowchart; the image is not this diagram type")
    return (
        f"<figure>{head}<div class='canvas {surface}'>"
        f"<img src='{uri}' alt='{esc(alt)}' style='width:{width}px' loading='lazy' decoding='async'></div>"
        f"<figcaption>{esc(' · '.join(caption))}</figcaption></figure>"
    )


def render_reference(row, spike, mmdc_failures, mmdc_version):
    inp = row["input"]
    head = '<p class="panel-title">Mermaid.js reference</p>'
    if inp["reference_svg"]:
        ref_path = spike / inp["reference_svg"]
        uri = svg_data_uri(ref_path)
        # Mermaid.js writes width="100%"; size the image from its viewBox like the PNGs.
        view_box = re.search(r'viewBox="[\d.\-]+ [\d.\-]+ ([\d.]+) [\d.]+"', ref_path.read_text()[:4000])
        width_style = f" style='width:{round(float(view_box.group(1)))}px'" if view_box else ""
        origin = "Merman fixture (Mermaid 12.2.0)" if inp["origin"] == "merman-fixture" else f"mmdc {mmdc_version}, default theme"
        return (
            f"<figure>{head}<div class='canvas ref'><img src=\"{uri}\"{width_style} alt='Mermaid.js reference for {esc(inp['name'])}' loading='lazy' decoding='async'></div>"
            f"<figcaption>{esc(origin)}</figcaption></figure>"
        )
    if inp["name"] in mmdc_failures:
        return f"<figure>{head}<div class='failure'>Mermaid.js ({esc(mmdc_version)}) rejects this input: {esc(mmdc_failures[inp['name']][-300:])}</div></figure>"
    return f"<figure>{head}<div class='neutral-box'>No reference: Merman stores none for this fixture.</div></figure>"


def render_input(row, theme, spike, store, mmdc_failures, mmdc_version):
    inp = row["input"]
    origin_label = {
        "merman-fixture": "Merman fixture",
        "repo-test": "Our tests",
        "gitgraph-case": "gitGraph case (R-S7)",
    }[inp["origin"]]
    source_ref = inp["source_ref"]
    if inp["origin"] == "merman-fixture":
        source_ref = source_ref.replace("merman/", "Merman: ", 1)
    panels = "".join(render_panel(row, theme, k, l, store, mmdc_failures) for k, l in RENDERERS)
    panels += render_reference(row, spike, mmdc_failures, mmdc_version)
    extra = ""
    gg = row.get("gitgraph")
    if gg and gg.get("merman_commits"):
        merges = [c for c in gg["merman_commits"] if c.get("parents") and len(c["parents"]) > 1]
        if merges:
            extra = "<p class='source-ref'>Merman layout data, merge commits: " + esc(
                "; ".join(f"{c['id']} ← {', '.join(c['parents'])} (tags: {', '.join(c.get('tags') or []) or 'none'})" for c in merges)
            ) + "</p>"
    return (
        f"<article class='input' id='{esc(inp['name'])}'>"
        f"<div class='input-head'><h3>{esc(inp['name'])}</h3><span class='pill pill-unsup'>{esc(origin_label)}</span></div>"
        f"<p class='source-ref'>{esc(source_ref)}</p>{extra}"
        f"<details><summary>Mermaid source</summary><pre><code>{esc(inp_text(spike, inp))}</code></pre></details>"
        f"<div class='panels'>{panels}</div></article>"
    )


def inp_text(spike, inp):
    return (spike / inp["mmd_path"]).read_text()


SURVEY_METRICS = [
    ("clipped", "Labels clipped"),
    ("zero_area_gradient", "Zero-area gradient paint"),
    ("backing_narrower", "Text past its shape"),
    ("outside_viewbox", "Text outside viewBox"),
    ("font_unmatched", "Requested font unmatched"),
]


def defect_survey(rows, mmdc_failures):
    """Per family, summed over valid inputs and both themes: raw Merman vs fixed."""
    out = {}
    for row in rows:
        inp = row["input"]
        if inp["origin"] == "repo-test" and inp["name"] in mmdc_failures:
            continue
        fam = out.setdefault(inp["family"], {
            "labels": 0, "raw": {k: 0 for k, _ in SURVEY_METRICS}, "fixed": {k: 0 for k, _ in SURVEY_METRICS},
            "custom": {k: 0 for k, _ in SURVEY_METRICS},
            "font_case_only": {"raw": 0, "fixed": 0, "custom": 0}, "worst_clip_px": 0.0, "worst_clip_pct": 0.0,
            "worst_real_clip_px": 0.0, "examples": [], "real_examples": [], "backing_examples": [],
            "fonts": {"raw": {}, "fixed": {}, "custom": {}},
        })
        for theme_res in row["results"].values():
            for key, side in (("merman", "raw"), ("merman-real", "fixed"), ("merman-native", "custom")):
                d = (theme_res.get(key) or {}).get("defects")
                if not d:
                    continue
                if side == "raw":
                    fam["labels"] += d["labels"]
                    if d["worst_clip_px"] > fam["worst_clip_px"]:
                        fam["worst_clip_px"] = d["worst_clip_px"]
                    fam["worst_clip_pct"] = max(fam["worst_clip_pct"], d["worst_clip_pct"])
                    for e in d["clip_examples"]:
                        if len(fam["examples"]) < 3 and e not in fam["examples"]:
                            fam["examples"].append(e)
                elif side == "fixed":
                    fam["worst_real_clip_px"] = max(fam["worst_real_clip_px"], d["worst_clip_px"])
                    for e in d["clip_examples"]:
                        if len(fam["real_examples"]) < 2 and e not in fam["real_examples"]:
                            fam["real_examples"].append(e)
                    for e in d["backing_examples"]:
                        if len(fam["backing_examples"]) < 2 and e not in fam["backing_examples"]:
                            fam["backing_examples"].append(e)
                for k, _ in SURVEY_METRICS:
                    fam[side][k] += d[k]
                fam["font_case_only"][side] += d["font_case_only"]
                for pair, n in d["fonts"].items():
                    fam["fonts"][side][pair] = fam["fonts"][side].get(pair, 0) + n
    return out


def render_survey(data, family_order, observations, real_fonts):
    parts = [
        "<h2 id='defect-survey'>Defect survey</h2>",
        "<p class='family-note'>Measured from usvg's parse of every Merman SVG, with the font database that column is drawn with, "
        "over valid inputs and both themes, so each label counts once per theme. Each cell is <strong>default measurement → real fonts</strong>: "
        "Merman's built-in deterministic measurement drawn with the system fonts and usvg's default resolver, against the real-fonts column: "
        "Merman measuring through our TextMeasurer and drawn by our rasterizer, both resolving each label's face with the same function "
        "(MERMAID_FONT, the label's font stack ignoring case, the OS's generic family via fontique, then an OS/2-classified face). "
        "No overflow fix is applied to either. "
        "Clipped: a label's glyph outline extends more than 0.5px past any ancestor clip. Zero-area gradient paint: a straight line painted by an "
        "objectBoundingBox gradient, which draws nothing (the real-fonts column applies the gradient fix). Text past its shape: the label extends "
        "more than 1px beyond the left or right edge of the smallest filled shape behind it. Requested font unmatched: the label's first font family "
        "names no installed face exactly (fontdb matches case-sensitively; the real-fonts column resolves names case-insensitively, as CSS does).</p>",
    ]
    for host, fams in data.items():
        host_label = {'macos': 'macOS', 'linux': 'Linux'}.get(host, host)
        head = "<tr><th>Family</th><th class='num'>Labels</th>" + "".join(f"<th class='num'>{esc(l)}</th>" for k, l in SURVEY_METRICS if k != "font_unmatched") + \
            "<th class='num'>Worst clip, default → real</th><th>Examples (default measurement)</th></tr>"
        body = []
        tot = {"labels": 0, "raw": {k: 0 for k, _ in SURVEY_METRICS}, "fixed": {k: 0 for k, _ in SURVEY_METRICS}}
        for f in family_order:
            d = fams.get(f)
            if not d:
                continue
            tot["labels"] += d["labels"]
            cells = []
            for k, _ in SURVEY_METRICS:
                if k == "font_unmatched":
                    continue
                r, x = d["raw"][k], d["fixed"][k]
                tot["raw"][k] += r
                tot["fixed"][k] += x
                cls = "better" if x < r else ("worse" if x > r else "")
                cells.append(f"<td class='num delta'>{r} → <span class='{cls}'>{x}</span></td>" if (r or x) else "<td class='num delta'>0</td>")
            worst = f"{d['worst_clip_px']:.1f} → {d['worst_real_clip_px']:.1f}px" if (d["worst_clip_px"] or d["worst_real_clip_px"]) else ""
            ex = "; ".join(d["examples"][:2] + [f"real fonts: {e}" for e in d["real_examples"][:1]] + d["backing_examples"][:1])
            body.append(f"<tr><td>{esc(FAMILY_LABEL.get(f, f))}</td><td class='num'>{d['labels']}</td>{''.join(cells)}"
                        f"<td class='num'>{esc(worst)}</td><td class='note'>{esc(ex)}</td></tr>")
        foot = "<tr><td>All</td><td class='num'>" + str(tot["labels"]) + "</td>" + "".join(
            f"<td class='num delta'>{tot['raw'][k]} → {tot['fixed'][k]}</td>" for k, _ in SURVEY_METRICS if k != "font_unmatched") + "<td></td><td></td></tr>"
        parts.append(f"<h3 class='survey-host'>{esc(host_label)}</h3>")
        parts.append(f"<div class='table-wrap'><table><thead>{head}</thead><tbody>{''.join(body)}</tbody><tfoot>{foot}</tfoot></table></div>")
        ch = real_fonts.get(host)
        if ch:
            step_label = {"mermaid-font": "MERMAID_FONT", "stack": "stack, case-insensitive", "stack-generic": "stack generic → OS answer",
                          "generic-fallback": "OS generic answer (step 3)", "first-face": "first installed face"}
            req = "; ".join(f"{name} → {face or 'none'} ({step_label.get(step, step)})" for name, step, face in ch["requested"])
            def g(c):
                origin = {"fontique": "fontique", "os2": "OS/2 classification", "none": "no answer"}[c["source"]]
                miss = f"; fontique's {c['fontique_family']} is not in usvg's fontdb" if c.get("in_fontdb") is False else ""
                return f"{c['family'] or 'none'} ({origin}{miss})"
            mf = ch.get("mermaid_font")
            parts.append(
                f"<p class='family-note'>Font chain on {esc(host_label)}: sans-serif → {esc(g(ch['sans_serif']))}, serif → {esc(g(ch['serif']))}, "
                f"monospace → {esc(g(ch['monospace']))}. Requested families: {esc(req)}. "
                f"MERMAID_FONT {'unset (step 1 skipped)' if not mf else esc(mf)}. fontique's system scan took {ch['fontique_ms']:.0f} ms once per process.</p>")
        fonts = {}
        for d in fams.values():
            for side in ("raw", "fixed"):
                for pair, n in d["fonts"][side].items():
                    fonts.setdefault(pair, {"raw": 0, "fixed": 0})[side] += n
        rows_f = "".join(f"<tr><td class='mono'>{esc(pair)}</td><td class='num'>{v['raw']}</td><td class='num'>{v['fixed']}</td></tr>"
                         for pair, v in sorted(fonts.items(), key=lambda kv: -kv[1]["raw"] - kv[1]["fixed"]))
        parts.append("<p class='family-note'>Face that drew each label on " + esc(host_label) + " (first requested family → face; label counts).</p>"
                     f"<div class='table-wrap'><table><thead><tr><th>Requested → drawn with</th><th class='num'>Default measurement column</th><th class='num'>Real fonts column</th></tr></thead><tbody>{rows_f}</tbody></table></div>")
    custom_rows = []
    for host, fams in data.items():
        host_label = {'macos': 'macOS', 'linux': 'Linux'}.get(host, host)
        t = {side: {k: sum(d[side][k] for d in fams.values()) for k, _ in SURVEY_METRICS} for side in ("raw", "fixed", "custom")}
        for side, label in (("raw", "Merman default measurement"), ("fixed", "Our TextMeasurer + font chain (gallery column)"), ("custom", "Merman's native measurement on the chain's font files")):
            custom_rows.append(f"<tr><td>{esc(host_label)}</td><td>{esc(label)}</td>" + "".join(
                f"<td class='num'>{t[side][k]}</td>" for k, _ in SURVEY_METRICS if k != "font_unmatched") + "</tr>")
    parts.append(
        "<h3 id='measurers'>Measurer comparison</h3><p class='family-note'>Totals over all inputs and both themes. The gallery's real-fonts column measures "
        "through our TextMeasurer, installed with Merman's public TextMeasurementProfile API, shaping each label with the rasterizer's resvg 0.45 font database "
        "and the same resolution function; Merman's wrapping helper is not public, so it wraps with a plain greedy word wrap. For comparison, Merman's own "
        "native measurement (native-fonts feature, NativeFontContext) on the font files of the faces the chain resolved, drawn with a database of exactly those files.</p>"
        "<div class='table-wrap'><table><thead><tr><th>Host</th><th>Measurement</th>" + "".join(
            f"<th class='num'>{esc(l)}</th>" for k, l in SURVEY_METRICS if k != "font_unmatched") + f"</tr></thead><tbody>{''.join(custom_rows)}</tbody></table></div>")
    remaining = observations.get("remaining_defects", [])
    if remaining:
        parts.append("<p class='family-note'>What remains with real fonts:</p><ul class='notes'>" +
                     "".join(f"<li>{esc(n)}</li>" for n in remaining) + "</ul>")
    return "".join(parts)


def measurement_cost(reports, mmdc_failures):
    """Per family and host: median of per-input medians (dark theme) for each Merman measurement."""
    out = {}
    for rep in reports:
        fams = {}
        for row in rep["rows"]:
            inp = row["input"]
            if inp["origin"] == "repo-test" and inp["name"] in mmdc_failures:
                continue
            d = row["results"].get("dark", {})
            entry = fams.setdefault(inp["family"], {"merman": [], "merman-real": [], "merman-real-warm": [], "merman-native": [], "merman-native-warm": []})
            for key in ("merman", "merman-real", "merman-native"):
                res = d.get(key) or {}
                if res.get("timing"):
                    entry[key].append(res["timing"]["median_ms"])
                if res.get("timing_warm"):
                    entry[f"{key}-warm"].append(res["timing_warm"]["median_ms"])
        out[rep["os"]] = {
            "native_context_load_ms": rep.get("native_context_load_ms"),
            "families": {f: {k: (statistics.median(v) if v else None) for k, v in e.items()} for f, e in fams.items()},
        }
    return out


def render_cost(cost, family_order):
    parts = ["<h3 id='measurement-cost'>Cost of real-font measurement</h3>",
             "<p class='family-note'>Median of per-input render medians, dark theme, 25 runs, SVG only (no rasterization). "
             "Cold: a fresh measurer (or, for Merman's native measurement, a freshly loaded font context) per render, so no shaping results "
             "are cached; creating it is outside the timed region. Warm: one measurer reused, as a long-running process would. The font chain's "
             "one-time setup (system font scan and fontique query) is not in these figures. Real-font figures include the gradient-stroke fix.</p>"]
    for host, c in cost.items():
        host_label = {'macos': 'macOS', 'linux': 'Linux'}.get(host, host)
        load = c.get("native_context_load_ms")
        rows = []
        for f in family_order:
            e = c["families"].get(f)
            if not e or e["merman"] is None:
                continue
            ratio = (e["merman-real"] / e["merman"]) if (e["merman-real"] and e["merman"]) else None
            rows.append(f"<tr><td>{esc(FAMILY_LABEL.get(f, f))}</td><td class='num'>{fmt_ms(e['merman'])}</td><td class='num'>{fmt_ms(e['merman-real'])}</td>"
                        f"<td class='num'>{fmt_ms(e['merman-real-warm'])}</td><td class='num'>{'n/a' if ratio is None else f'{ratio:.1f}×'}</td>"
                        f"<td class='num'>{fmt_ms(e['merman-native'])}</td><td class='num'>{fmt_ms(e['merman-native-warm'])}</td></tr>")
        parts.append(f"<p class='family-note'>{esc(host_label)}: loading Merman's native font context takes {fmt_ms(load)} ms.</p>"
                     "<div class='table-wrap'><table><thead><tr><th>Family</th><th class='num'>Default ms</th><th class='num'>Real fonts, cold ms</th>"
                     "<th class='num'>Real fonts, warm ms</th><th class='num'>Cold / default</th><th class='num'>Merman native, cold ms</th><th class='num'>Merman native, warm ms</th></tr></thead>"
                     f"<tbody>{''.join(rows)}</tbody></table></div>")
    return "".join(parts)


CLI_METRICS = [("clipped", "Clipped"), ("zero_area_gradient", "Zero-area gradient"), ("backing_narrower", "Past its shape"), ("outside_viewbox", "Outside viewBox")]


def cli_survey(path):
    """Per family, summed over both themes: merman-cli deterministic vs native measurement."""
    if not path or not Path(path).exists():
        return None
    rows = json.loads(Path(path).read_text())
    out = {"identical_to_library_svg": [sum(1 for r in rows if r.get("identical_to_library_svg")),
                                        sum(1 for r in rows if "identical_to_library_svg" in r)], "families": {}}
    for r in rows:
        fam = out["families"].setdefault(r["family"], {m: {k: 0 for k, _ in CLI_METRICS} | {"labels": 0} for m in ("deterministic", "native")})
        for mode in ("deterministic", "native"):
            d = r.get(mode) or {}
            if "error" in d or not d:
                continue
            fam[mode]["labels"] += d["labels"]
            for k, _ in CLI_METRICS:
                fam[mode][k] += d[k]
    return out


def ci_comparison(reports):
    """Totals per host: default font resolution vs case-insensitive, raw and fixed."""
    out = {}
    for rep in reports:
        tot = {}
        for row in rep["rows"]:
            for theme_res in row["results"].values():
                for key in ("merman", "merman-fixed"):
                    res = theme_res.get(key) or {}
                    for var in ("defects", "defects_ci"):
                        d = res.get(var)
                        if not d:
                            continue
                        t = tot.setdefault(f"{key}/{var}", {k: 0 for k, _ in CLI_METRICS} | {"worst_clip_px": 0.0})
                        for k, _ in CLI_METRICS:
                            t[k] += d[k]
                        t["worst_clip_px"] = max(t["worst_clip_px"], d["worst_clip_px"])
        out[rep["os"]] = tot
    return out


def render_cli_and_ci(cli, ci, family_order):
    parts = []
    if cli:
        same, total = cli["identical_to_library_svg"]
        parts.append(
            "<h3 id='merman-cli'>Merman's own export (merman-cli, macOS)</h3>"
            "<p class='family-note'>Each Merman fixture and gitGraph case rendered by merman-cli at the same rev with "
            "<code>--svg-pipeline resvg-safe</code> (the SVG its PNG export rasterizes), dark and default themes, surveyed with a "
            "case-insensitive resolver like Merman's export uses. Deterministic is Merman's default text measurement; native is "
            "<code>--font-measurement native</code> with the requested faces passed as <code>--font-file</code> (system discovery fails on macOS "
            f"with a font-size limit). {same} of {total} deterministic SVGs are byte-identical to the library SVGs in the Merman column; the rest differ in "
            "layout geometry (the CLI build draws, for example, orthogonal class edges). A non-zero deterministic count means the defect reproduces "
            "in Merman's own export, with no involvement from this spike.</p>")
        head = "<tr><th>Family</th>" + "".join(f"<th class='num'>{esc(l)}<br>deterministic → native</th>" for _, l in CLI_METRICS) + "<th>Reproduces in Merman's export</th></tr>"
        body = []
        for f in family_order:
            d = cli["families"].get(f)
            if not d:
                continue
            det, nat = d["deterministic"], d["native"]
            cells = "".join(f"<td class='num delta'>{det[k]} → {nat[k]}</td>" if (det[k] or nat[k]) else "<td class='num delta'>0</td>" for k, _ in CLI_METRICS)
            repro = ", ".join(l.lower() for k, l in CLI_METRICS if det[k]) or "none"
            body.append(f"<tr><td>{esc(FAMILY_LABEL.get(f, f))}</td>{cells}<td class='note'>{esc(repro)}</td></tr>")
        foot = "<tr><td>All</td>" + "".join(
            f"<td class='num delta'>{sum(v['deterministic'][k] for v in cli['families'].values())} → {sum(v['native'][k] for v in cli['families'].values())}</td>"
            for k, _ in CLI_METRICS) + "<td></td></tr>"
        parts.append(f"<div class='table-wrap'><table><thead>{head}</thead><tbody>{''.join(body)}</tbody><tfoot>{foot}</tfoot></table></div>")
    if ci:
        parts.append(
            "<h3 id='case-insensitive'>Case-insensitive font resolution (candidate fix d)</h3>"
            "<p class='family-note'>Every Merman SVG surveyed twice: with usvg's default resolver and with a case-insensitive resolver modeled on Merman's "
            "export, which resolves \"trebuchet ms\" to Trebuchet MS and \"courier\" to Courier where installed. Totals over all inputs and both themes. "
            "It is not applied to the fixed column, because it does not reduce label overflow (see the counts).</p>")
        head = "<tr><th>Host</th><th>Column / resolver</th>" + "".join(f"<th class='num'>{esc(l)}</th>" for _, l in CLI_METRICS) + "<th class='num'>Worst clip px</th></tr>"
        labels = {"merman/defects": "Merman, default", "merman/defects_ci": "Merman, case-insensitive",
                  "merman-fixed/defects": "Merman + fixes, default", "merman-fixed/defects_ci": "Merman + fixes, case-insensitive"}
        body = []
        for host, tot in ci.items():
            for key, label in labels.items():
                t = tot.get(key)
                if not t:
                    continue
                body.append(f"<tr><td>{esc({'macos': 'macOS', 'linux': 'Linux'}.get(host, host))}</td><td>{esc(label)}</td>" +
                            "".join(f"<td class='num'>{t[k]}</td>" for k, _ in CLI_METRICS) + f"<td class='num'>{t['worst_clip_px']:.1f}</td></tr>")
        parts.append(f"<div class='table-wrap'><table><thead>{head}</thead><tbody>{''.join(body)}</tbody></table></div>")
    return "".join(parts)

def build(args):
    spike = Path(args.spike).resolve()
    report = json.loads(Path(args.results).read_text())
    rows = report["rows"]
    mmdc = json.loads((spike / "corpus" / "mmdc-results.json").read_text())
    mmdc_failures, mmdc_version = mmdc["failures"], mmdc["mmdc_version"]
    observations = json.loads((spike / "scripts" / "observations.json").read_text())
    store = PngStore(Path(args.png_cache))
    theme = args.theme
    families = family_summary(rows, theme, mmdc_failures)
    timing_families = family_summary(rows, "dark", mmdc_failures)
    order = [f for f in FAMILY_ORDER if f in families] + sorted(set(families) - set(FAMILY_ORDER))
    shared = [f for f in order if families[f]["group"] == "shared"]
    only = [f for f in order if families[f]["group"] == "merman-only"]

    theme_label = "Dark (biscuit-visualized MermaidTheme::Dark vs Merman site config theme: dark)" if theme == "dark" \
        else "Default/light (biscuit-visualized MermaidTheme::Default vs Merman site config theme: default)"
    here = Path(args.out).name
    variant_links = "".join(
        f"<span class='here'>{esc(label)}</span>" if name == here
        else f"<a href='{url}' target='_top'>{esc(label)}</a>"
        for name, (label, _title, url) in VARIANTS.items()
    )
    page_title = VARIANTS.get(here, ("", "Merman Bake-off", ""))[1]

    out = []
    out.append(f"<title>{esc(page_title)}</title>")
    out.append('<meta name="claude:viewport-fit-sides" content="cover">')
    out.append('<link rel="preconnect" href="https://fonts.googleapis.com"><link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>')
    out.append('<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&display=swap">')
    out.append(f"<style>{CSS}</style>")
    out.append("<main>")
    out.append("<h1>Merman Bake-off</h1>")
    out.append(
        "<p class='lede'>"
        f"Each input is rendered by today's <code>biscuit-visualized</code> (mermaid-rs-renderer 0.3.1 with our corrections and themes) "
        f"and by Merman @ <code>{REV_SHORT}</code> with its out-of-the-box Mermaid theme, both rasterized by the same resvg 0.45.1 path. "
        f"Theme pairing on this page: {esc(theme_label)}. Read each row left to right: current, Merman, then the Mermaid.js reference "
        "(always the light default theme), which is the fidelity target. Timings are warm, in-process, release-build SVG render times "
        f"(no rasterization), median and p95 of {report['iterations']} runs, measured on the dark theme. Rendered 2026-10-21 on {esc(report['host'])}."
        "</p>"
    )
    out.append(f"<div class='variants'>Variants: {variant_links}</div>")
    nav = ["<nav class='families' aria-label='Families'><a href='#scorecard'>Scorecard</a><span class='sep'>|</span>"]
    nav += [f"<a href='#fam-{f}'>{esc(FAMILY_LABEL.get(f, f))}</a>" for f in shared]
    nav.append("<span class='sep'>| Merman only:</span>")
    nav += [f"<a href='#fam-{f}'>{esc(FAMILY_LABEL.get(f, f))}</a>" for f in only]
    nav.append("<a href='#defect-survey'>Defect survey</a><a href='#methodology'>Methodology</a></nav>")
    out.append("".join(nav))

    # Scorecard
    out.append("<h2 id='scorecard'>Scorecard</h2>")
    out.append(
        "<p class='family-note'>Status counts only valid inputs (inputs Mermaid.js itself rejects are excluded and shown per input). "
        "Family median is the median of per-input medians over inputs both renderers drew; family p95 is the largest per-input p95. "
        "Ratio is Merman median / current median. Notes are factual observations from this run, not a verdict.</p>"
    )
    head = ("<tr><th>Family</th><th class='num'>Inputs</th><th>Current</th><th>Merman</th><th>Merman (real fonts)</th>"
            "<th class='num'>Current median ms</th><th class='num'>of which cache I/O</th><th class='num'>p95</th><th class='num'>Merman median ms</th><th class='num'>p95</th>"
            "<th class='num'>Merman / current</th><th class='num'>… net of cache I/O</th><th class='num'>Real fonts median ms</th><th>Observed</th><th>Observed with real fonts</th><th>Ken's verdict</th></tr>")
    body = []
    totals = {"both": 0, "merman_only": 0, "current_fail": 0, "merman_fail": 0}
    for f in shared + only:
        fs = families[f]
        tf = timing_families[f]
        st = fs["status"]
        if fs["group"] == "shared" and st["current"] == "rendered" and st["merman"] == "rendered":
            totals["both"] += 1
        if fs["group"] == "merman-only" and st["merman"] == "rendered":
            totals["merman_only"] += 1
        if st["current"] in ("failed", "partial"):
            totals["current_fail"] += 1
        if st["merman"] in ("failed", "partial"):
            totals["merman_fail"] += 1

        def status_cell(key):
            s = st[key]
            label = None
            if s == "partial":
                label = f"partial {fs['ok'][key]}/{fs['valid_inputs']}"
            return pill(s, label)

        ratio, ratio_net = tf["ratio"], tf["ratio_net"]
        body.append(
            f"<tr><td><a href='#fam-{f}'>{esc(FAMILY_LABEL.get(f, f))}</a></td><td class='num'>{fs['inputs']}</td>"
            f"<td>{status_cell('current')}</td><td>{status_cell('merman')}</td><td>{status_cell('merman-real')}</td>"
            f"<td class='num'>{fmt_ms(tf['median_ms']['current'])}</td><td class='num'>{fmt_ms(tf['cache_io_ms'])}</td><td class='num'>{fmt_ms(tf['p95_ms']['current'])}</td>"
            f"<td class='num'>{fmt_ms(tf['median_ms']['merman'])}</td><td class='num'>{fmt_ms(tf['p95_ms']['merman'])}</td>"
            f"<td class='num'>{'n/a' if ratio is None else f'{ratio:.2f}×'}</td>"
            f"<td class='num'>{'n/a' if ratio_net is None else f'{ratio_net:.1f}×'}</td>"
            f"<td class='num'>{fmt_ms(tf['median_ms'].get('merman-real'))}</td>"
            f"<td class='note'>{esc(observations.get('families', {}).get(f, ''))}</td>"
            f"<td class='note'>{esc(observations.get('real_fonts', {}).get(f, ''))}</td><td class='verdict'></td></tr>"
        )
    foot = (
        f"<tr><td colspan='16'>Families both render: {totals['both']} of {len(shared)} shared · "
        f"families only Merman renders: {totals['merman_only']} of {len(only)} · "
        f"families with a failure: current {totals['current_fail']}, Merman {totals['merman_fail']}</td></tr>"
    )
    out.append(f"<div class='table-wrap'><table><thead>{head}</thead><tbody>{''.join(body)}</tbody><tfoot>{foot}</tfoot></table></div>")

    def family_section(f):
        fs = families[f]
        rows_f = sorted((r for r in rows if r["input"]["family"] == f),
                        key=lambda r: (ORIGIN_ORDER[r["input"]["origin"]], r["input"]["name"]))
        note = observations.get("families", {}).get(f, "")
        parts = [f"<section class='family' id='fam-{f}'><h2>{esc(FAMILY_LABEL.get(f, f))}</h2>"]
        if note:
            parts.append(f"<p class='family-note'>{esc(note)}</p>")
        parts += [render_input(r, theme, spike, store, mmdc_failures, mmdc_version) for r in rows_f]
        parts.append("</section>")
        return "".join(parts)

    out += [family_section(f) for f in shared]
    out.append("<h2 id='merman-only'>Merman-only families</h2>")
    out.append("<p class='family-note'>mermaid-rs-renderer has no parser for these families. For most it does not fail: text that looks like flowchart edges falls back to a flowchart, so the current column shows a wrong diagram rather than an error. They are shown for completeness.</p>")
    out += [family_section(f) for f in only]

    # Defect survey
    other_report = json.loads(Path(args.other_results).read_text()) if args.other_results and Path(args.other_results).exists() else None
    survey_hosts = [report] + ([other_report] if other_report else [])
    survey_data = {h["os"]: defect_survey(h["rows"], mmdc_failures) for h in survey_hosts}
    real_fonts = {h["os"]: h.get("font_chain") for h in survey_hosts}
    out.append(render_survey(survey_data, shared + only, observations, real_fonts))
    cost_data = measurement_cost(survey_hosts, mmdc_failures)
    out.append(render_cost(cost_data, shared + only))
    cli_data = cli_survey(args.merman_cli)
    out.append(render_cli_and_ci(cli_data, None, shared + only))
    if args.font_demo and Path(args.font_demo).exists():
        demos = json.loads(Path(args.font_demo).read_text())
        figs = []
        for d in demos:
            label = f"{d['host']}: MERMAID_FONT " + ("unset" if not d.get("mermaid_font") else repr(d["mermaid_font"]))
            drawn = ", ".join(f"{k} ({v})" for k, v in (d.get("drawn_with") or {}).items())
            note = d.get("warning") or ("override applied" if d.get("mermaid_font_installed") else ("step 1 skipped" if not d.get("mermaid_font") else ""))
            figs.append(f"<figure><p class='panel-title'>{esc(label)}</p><div class='canvas light'><img src='{store.data_uri(d['png'])}' alt='{esc(label)}' style='width:{d.get('width', 300)}px'></div>"
                        f"<figcaption>{esc('drawn with ' + drawn + (' · ' + note if note else ''))}</figcaption></figure>")
        out.append("<h3 id='mermaid-font'>MERMAID_FONT</h3><p class='family-note'>One small flowchart through the font chain. Unset or empty: step 1 is skipped silently. "
                   "Set to an installed family: it is matched ignoring case and used for every label. Set to a family that is not installed: one warning naming it, "
                   "then the Mermaid font stack as usual.</p>" + f"<div class='panels'>{''.join(figs)}</div>")

    # Methodology and build weight
    out.append("<h2 id='methodology'>Build weight and methodology</h2>")
    if args.build_weight and Path(args.build_weight).exists():
        bw = json.loads(Path(args.build_weight).read_text())
        rows_bw = "".join(
            f"<tr><td>{esc(v['variant'])}</td><td class='mono'>{esc(v['flags'] or '(default: both)')}</td>"
            f"<td class='num'>{v['build_seconds']}</td><td class='num'>{v['binary_bytes'] / 1048576:.1f}</td>"
            f"<td class='num'>{v['stripped_bytes'] / 1048576:.1f}</td><td class='num'>{v['unique_packages']}</td></tr>"
            for v in bw
        )
        out.append(
            "<p class='family-note'>Uncached release builds of the spike binary (same <code>main.rs</code>, one renderer compiled in), each in a fresh target dir, "
            "no compiler cache, sources pre-fetched. The binary also contains resvg 0.45, serde, and the corpus loader. Information only (D29).</p>"
            "<div class='table-wrap'><table><thead><tr><th>Variant</th><th>Cargo flags</th><th class='num'>Build s</th>"
            "<th class='num'>Binary MB</th><th class='num'>Stripped MB</th><th class='num'>Packages (cargo tree)</th></tr></thead>"
            f"<tbody>{rows_bw}</tbody></table></div>"
        )
    if args.cross_os and Path(args.cross_os).exists():
        co = json.loads(Path(args.cross_os).read_text())
        out.append(f"<p class='notes'>Cross-OS PNG difference (R-S9), macOS vs Linux: {esc(co['summary'])}</p>")
    notes = list(observations.get("methodology", []))
    other = json.loads(Path(args.other_results).read_text()) if args.other_results and Path(args.other_results).exists() else None
    hosts = [report] + ([other] if other else [])
    notes.append(
        "Generic font families (both columns, rasterizer-side fallback planned as D24): after loading system fonts, "
        "sans-serif, serif, and monospace are each pointed at the first installed family from a fixed candidate list. "
        "Resolved: " + "; ".join(
            f"{h['os']}: sans-serif → {g.get('sans_serif')}, serif → {g.get('serif')}, monospace → {g.get('monospace')}"
            for h in hosts for g in [h.get("generic_families") or {}]
        ) + ". Without it, fontdb's fixed defaults (Arial, Times New Roman, Courier New) do not exist on the Linux host "
        "and resvg silently dropped every Merman label there."
    )
    out.append("<ul class='notes'>" + "".join(f"<li>{esc(n)}</li>" for n in notes) + "</ul>")
    out.append("</main>")
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    Path(args.out).write_text(scrub_home("\n".join(out)))
    size = Path(args.out).stat().st_size
    print(f"wrote {args.out}: {size / 1048576:.2f} MB")

    if args.scorecard:
        other = json.loads(Path(args.other_results).read_text()) if args.other_results and Path(args.other_results).exists() else None
        card = {
            "date": "2026-10-21",
            "merman_rev": report["merman_rev"],
            "host": report["host"],
            "os": report["os"],
            "iterations": report["iterations"],
            "theme_pairing": {
                "dark": "biscuit-visualized MermaidTheme::Dark vs Merman site config {\"theme\": \"dark\"}",
                "default": "biscuit-visualized MermaidTheme::Default vs Merman site config {\"theme\": \"default\"}",
            },
            "timing_theme": "dark",
            "mmdc_version": mmdc_version,
            "families": [
                {k: v for k, v in timing_families[f].items() if k not in ("medians", "p95s")}
                | {"status": families[f]["status"], "observed": observations.get("families", {}).get(f, ""),
                   "observed_with_real_fonts": observations.get("real_fonts", {}).get(f, "")}
                for f in shared + only
            ],
            "totals": totals,
            "defect_survey": survey_data,
            "merman_cli_survey": cli_data,
            "measurement_cost": cost_data,
            "font_chain": real_fonts,
            "native_font_files": {h["os"]: h.get("real_fonts") for h in survey_hosts},
            "font_demo": json.loads(Path(args.font_demo).read_text()) if args.font_demo and Path(args.font_demo).exists() else None,
            "inputs": [
                {
                    "input": {k: v for k, v in r["input"].items()},
                    "mermaid_js_rejects": r["input"]["name"] in mmdc_failures,
                    "results": r["results"],
                    "gitgraph": r.get("gitgraph"),
                }
                for r in rows
            ],
            "build_weight": json.loads(Path(args.build_weight).read_text()) if args.build_weight and Path(args.build_weight).exists() else None,
            "cross_os": json.loads(Path(args.cross_os).read_text()) if args.cross_os and Path(args.cross_os).exists() else None,
            "other_os": None if other is None else {
                "os": other["os"], "host": other["host"],
                "timings": {
                    r["input"]["name"]: {
                        k: (v.get("timing") if v else None) for k, v in r["results"].get("dark", {}).items()
                    }
                    for r in other["rows"]
                },
            },
        }
        # PNG paths point into a scratch dir; keep names only.
        for item in card["inputs"]:
            for theme_res in item["results"].values():
                for res in theme_res.values():
                    if res.get("png"):
                        res["png"] = Path(res["png"]).name
        Path(args.scorecard).write_text(scrub_home(json.dumps(card, indent=2) + "\n"))
        print(f"wrote {args.scorecard}")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--spike", required=True)
    p.add_argument("--results", required=True)
    p.add_argument("--theme", default="dark", choices=["dark", "default"])
    p.add_argument("--out", required=True)
    p.add_argument("--scorecard")
    p.add_argument("--build-weight")
    p.add_argument("--cross-os")
    p.add_argument("--other-results")
    p.add_argument("--png-cache", default="/tmp/s1-png-cache")
    p.add_argument("--merman-cli", help="merman_cli_survey.py output")
    p.add_argument("--font-demo", help="JSON describing MERMAID_FONT demo renders (with PNG paths)")
    build(p.parse_args())


if __name__ == "__main__":
    main()

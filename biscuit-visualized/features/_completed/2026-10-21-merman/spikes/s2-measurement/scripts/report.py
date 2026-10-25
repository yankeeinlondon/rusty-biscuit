#!/usr/bin/env python3
"""Spike S2 report: combines the `analyze` results, Mermaid.js references, and
timing into `results.json`, prints the scorecard tables (Markdown) used in
FINDINGS.md, and writes `gallery/index.html`.

Usage:
  python3 -I scripts/report.py --spike . \
      --host macos <analyze-out> <refs-dir> <timing.json> \
      --host linux <analyze-out> <refs-dir> <timing.json>
"""

import argparse
import base64
import html
import json
import math
import re
import statistics
import subprocess
from pathlib import Path

MEASURERS = ["a", "a-mw", "b"]
MEASURER_LABEL = {
    "a": "A (rustybuzz, greedy wrap)",
    "a-mw": "A + Merman wrap",
    "b": "B (Merman native)",
}


def norm(text: str) -> str:
    return re.sub(r"\s+", " ", text.replace("­", "")).strip()


def is_nan(x) -> bool:
    return x is None or (isinstance(x, float) and math.isnan(x))


# ------------------------------------------------------------------ accuracy


def accuracy(rows, measurer, sets=None):
    diffs, declined, not_compared = [], 0, 0
    for row in rows:
        if sets and row["input"]["set"] not in sets:
            continue
        res = row["results"].get(measurer)
        if not res or not res["ok"] or not res.get("analysis"):
            continue
        for label in res["analysis"]["labels"]:
            if not label["compared"]:
                not_compared += 1
            elif is_nan(label["measured_width"]):
                declined += 1
            else:
                diffs.append(abs(label["diff"]))
    diffs.sort(reverse=True)
    n = len(diffs)
    return {
        "labels": n,
        "max_px": round(diffs[0], 3) if n else None,
        "p95_px": round(diffs[int(0.05 * n)], 3) if n else None,
        "above_1px": sum(d > 1 for d in diffs),
        "declined": declined,
        "not_compared": not_compared,
    }


def defects(rows, measurer, sets=None):
    total = {"clipped": 0, "worst_clip_px": 0.0, "past_box": 0, "off_canvas": 0, "zero_area_gradient": 0, "errors": 0}
    examples = []
    for row in rows:
        if sets and row["input"]["set"] not in sets:
            continue
        res = row["results"].get(measurer)
        if not res:
            continue
        if not res["ok"]:
            total["errors"] += 1
            continue
        d = (res.get("analysis") or {}).get("defects") or {}
        total["clipped"] += d.get("clipped", 0)
        total["worst_clip_px"] = round(max(total["worst_clip_px"], d.get("worst_clip_px", 0)), 2)
        total["past_box"] += d.get("backing_narrower", 0)
        total["off_canvas"] += d.get("outside_viewbox", 0)
        total["zero_area_gradient"] += d.get("zero_area_gradient", 0)
        for kind, key in (("clipped", "clip_examples"), ("past box", "backing_examples"), ("off canvas", "outside_examples")):
            for ex in d.get(key, []):
                examples.append(f"{row['input']['name']}: {kind} {ex}")
    total["examples"] = examples
    return total


def fallback_mismatches(rows, measurer):
    out = []
    for row in rows:
        res = row["results"].get(measurer)
        if not res or not res.get("analysis"):
            continue
        for label in res["analysis"]["labels"]:
            if label["compared"] and label["fallback_mismatches"]:
                out.append({"input": row["input"]["name"], "label": " / ".join(label["lines"]),
                            "mismatches": label["fallback_mismatches"][:6], "diff_px": round(label["diff"], 2)})
    return out


def drawn_fallback_faces(rows, measurer="b"):
    """Non-ASCII glyph -> face usvg drew, over the mixed set."""
    faces = {}
    for row in rows:
        if row["input"]["set"] != "mixed":
            continue
        res = row["results"].get(measurer)
        for label in (res.get("analysis") or {}).get("labels", []) if res else []:
            for entry in label["drawn_faces"]:
                glyph, _, face = entry.partition(":")
                faces.setdefault(face, set()).add(glyph)
    return {face: "".join(sorted(g))[:60] for face, g in sorted(faces.items())}


# ------------------------------------------------------------------ wrapping


def match_lines(groups, ref_lines):
    target = norm(" ".join(ref_lines))
    if not target:
        return None
    for group in groups:
        for i in range(len(group)):
            acc = ""
            for j in range(i, len(group)):
                acc = norm(acc + " " + group[j])
                if acc == target:
                    return [norm(x) for x in group[i:j + 1]]
                if len(acc) > len(target) or not target.startswith(acc):
                    break
    return None


def wrapping(rows, refs, measurer, sets):
    per_label = []
    for row in rows:
        if row["input"]["set"] not in sets:
            continue
        ref = refs["diagrams"].get(row["input"]["name"])
        res = row["results"].get(measurer)
        if not ref or "labels" not in ref or not res or not res.get("analysis"):
            continue
        groups = res["analysis"]["groups"]
        for label in ref["labels"]:
            ref_lines = [norm(x) for x in label["lines"]]
            ours = match_lines(groups, ref_lines)
            per_label.append({
                "input": row["input"]["name"],
                "set": row["input"]["set"],
                "family": row["input"]["family"],
                "reference": ref_lines,
                "ours": ours,
                "matched": ours is not None,
                "same_count": ours is not None and len(ours) == len(ref_lines),
                "same_breaks": ours == ref_lines,
            })
    return per_label


def wrap_summary(per_label, multi_only=False):
    """`multi_only`: labels Mermaid.js or we drew on more than one line."""
    rows = [x for x in per_label if x["matched"]]
    if multi_only:
        rows = [x for x in rows if len(x["reference"]) > 1 or len(x["ours"]) > 1]
    return {
        "labels": len(per_label) if not multi_only else len(rows),
        "matched": len(rows),
        "same_line_count": sum(x["same_count"] for x in rows),
        "same_breaks": sum(x["same_breaks"] for x in rows),
    }


# ------------------------------------------------------------------ timing


def med(xs):
    xs = [x for x in xs if not is_nan(x)]
    return statistics.median(xs) if xs else None


def timing_summary(timing, rows):
    family = {r["input"]["name"]: (r["input"]["set"], r["input"]["family"]) for r in rows}
    cold = {}
    for name, per in timing["cold_ms"].items():
        for m, xs in per.items():
            cold.setdefault(m, {})[name] = med(xs)
    warm = {}
    for s in timing["warm"]:
        warm.setdefault(s["measurer"], {}).setdefault(s["input"], []).append(s["render_ms"])
    warm = {m: {k: med(v) for k, v in per.items()} for m, per in warm.items()}
    out = {}
    for m in MEASURERS:
        c = [v for v in cold.get(m, {}).values() if v]
        w = [v for v in warm.get(m, {}).values() if v]
        out[m] = {
            "cold_median_of_medians_ms": round(med(c), 1) if c else None,
            "cold_max_median_ms": round(max(c), 1) if c else None,
            "warm_median_of_medians_ms": round(med(w), 2) if w else None,
            "warm_max_median_ms": round(max(w), 2) if w else None,
            "diagrams": len(c),
        }
    by_family = {}
    for name, (set_, fam) in family.items():
        if set_ == "newline":
            continue
        by_family[name] = {m: {"cold": cold.get(m, {}).get(name), "warm": warm.get(m, {}).get(name)} for m in MEASURERS}
    return out, by_family


def calls_summary(rows):
    out = {}
    for m in MEASURERS:
        per = [r["results"][m]["calls"]["total"] for r in rows if m in r["results"] and r["results"][m]["ok"]]
        shapes = [r["results"][m]["calls"].get("shapes") or 0 for r in rows if m in r["results"] and r["results"][m]["ok"]]
        out[m] = {"median_calls": med(per), "max_calls": max(per) if per else None, "total": sum(per),
                  "median_shapes": med(shapes) if m != "b" else None}
    return out


# ------------------------------------------------------------------ newline


def mjs_state(ref):
    if ref is None:
        return None, None
    if "error" in ref:
        return "rejected", None
    for label in ref.get("labels", []):
        if any("Alpha" in line or "Bravo" in line for line in label["lines"]):
            lines = label["lines"]
            if any("\\n" in line for line in lines):
                return "literal \\n", lines
            if len(lines) >= 2 and lines[0].strip() == "Alpha one":
                return "breaks", lines
            return "one line (\\n drawn as a space)" if len(lines) == 1 else f"{len(lines)} lines", lines
    return "label not found", None


def newline_table(rows, refs, refs12):
    out = []
    for row in rows:
        if row["input"]["set"] != "newline":
            continue
        name = row["input"]["name"]
        s11, l11 = mjs_state(refs["diagrams"].get(name))
        s12, l12 = mjs_state(refs12["diagrams"].get(name)) if refs12 else (None, None)
        entry = {"input": name, "family": row["input"]["family"],
                 "mermaid_js_11": s11, "mermaid_js_11_lines": l11,
                 "mermaid_js_12": s12, "mermaid_js_12_lines": l12}
        for nl in row["newline"]:
            m = nl["measurer"]
            for which in ("raw", "rewritten", "joined"):
                entry[f"{m}_{which}"] = nl[which]
            for which in ("raw", "rewritten"):
                entry[f"{m}_{which}_error"] = nl[f"{which}_error"]
                entry[f"{m}_{which}_png"] = nl[f"{which}_png"]
        out.append(entry)
    return out


def _box_state(case, joined, raw, rewritten):
    """Compares a breaking label's box with the one-line control."""
    box, one_box = case.get("box_width"), joined.get("box_width")
    if not box or not one_box:
        return "no box"
    boxes = [b for b in (raw.get("box_width"), rewritten.get("box_width"), one_box) if b]
    if max(boxes) - min(boxes) < 1:
        return "box unchanged (fixed or wrap-width box)"
    if box < one_box - 4:
        return "box fits widest line"
    return "box keeps one-line width"


def _lines_state(case):
    lines = case.get("lines") or []
    if not lines:
        return "label not found"
    if any("<br" in line for line in lines):
        return "<br/> drawn as text"
    if case.get("literal_backslash_n"):
        return "literal \\n"
    if any("onenBravo" in line.replace(" ", "") for line in lines):
        return "backslash dropped (\"onen\")"
    first = lines[0].strip()
    if len(lines) >= 2 and first.endswith("Alpha one"):
        return "breaks"
    return f"no break at \\n ({len(lines)} line{'s' if len(lines) > 1 else ''})"


def newline_verdict(entry, m):
    joined = entry.get(f"{m}_joined") or {}
    out = []
    for which in ("raw", "rewritten"):
        if entry.get(f"{m}_{which}_error"):
            out.append("Merman rejects")
            continue
        case = entry.get(f"{m}_{which}") or {}
        state = _lines_state(case)
        if state == "breaks":
            state += "; " + _box_state(case, joined, entry.get(f"{m}_raw") or {}, entry.get(f"{m}_rewritten") or {})
        out.append(state)
    return out[0], out[1]


# ------------------------------------------------------------------ gallery

CSS = r"""
:root{
  --bg:#f7f7f5;--surface:#ffffff;--ink:#1c1d1f;--muted:#5d6168;--line:#d9dad6;--accent:#b0127f;
  --chip:#eef0ec;--good:#1f7a3f;--bad:#b3261e;--panel:#1f2020;
  --font-sans:"IBM Plex Sans",-apple-system,"Segoe UI",system-ui,sans-serif;
  --font-mono:"IBM Plex Mono",ui-monospace,"SF Mono",Menlo,Consolas,monospace;
  color-scheme:light;
}
@media (prefers-color-scheme:dark){:root:not([data-theme="light"]){
  --bg:#141516;--surface:#1c1d1f;--ink:#e8e8e4;--muted:#a3a7ad;--line:#33363a;--accent:#f070c8;
  --chip:#26282b;--good:#6fcf8f;--bad:#ff8a80;color-scheme:dark;}}
:root[data-theme="dark"]{
  --bg:#141516;--surface:#1c1d1f;--ink:#e8e8e4;--muted:#a3a7ad;--line:#33363a;--accent:#f070c8;
  --chip:#26282b;--good:#6fcf8f;--bad:#ff8a80;color-scheme:dark;}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--ink);font:15px/1.5 var(--font-sans);
  padding-inline:max(16px,env(safe-area-inset-left)) max(16px,env(safe-area-inset-right));}
main{max-width:1500px;margin:0 auto;padding-block:28px 64px}
h1{font-size:1.6rem;margin:0 0 .3rem;letter-spacing:-.01em}
h2{font-size:1.2rem;margin:2.4rem 0 .6rem;border-bottom:1px solid var(--line);padding-bottom:.3rem}
h3{font-size:1rem;margin:1.6rem 0 .4rem}
p,li{color:var(--ink);max-width:80ch}
.lede{color:var(--muted)}
code,.mono{font-family:var(--font-mono);font-size:.88em}
.legend{display:inline-block;width:14px;height:10px;border:1px solid #ff3cdc;background:rgba(255,0,200,.18);vertical-align:middle;margin-right:4px}
.case{background:var(--surface);border:1px solid var(--line);border-radius:10px;padding:14px;margin:16px 0}
.case header{display:flex;flex-wrap:wrap;gap:8px;align-items:baseline;margin-bottom:10px}
.case header b{font-size:1rem}
.chip{background:var(--chip);border-radius:999px;padding:1px 9px;font-size:.8rem;color:var(--muted)}
.panels{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px}
@media (max-width:900px){.panels{grid-template-columns:1fr}}
figure{margin:0}
figure img{display:block;width:100%;height:auto;background:var(--panel);border-radius:6px;border:1px solid var(--line)}
figcaption{font-size:.82rem;color:var(--muted);margin-top:4px}
.scroll{overflow-x:auto}
table{border-collapse:collapse;font-size:.85rem;margin:8px 0 4px;min-width:640px}
th,td{border-bottom:1px solid var(--line);padding:5px 8px;text-align:left;vertical-align:top}
th{color:var(--muted);font-weight:600}
td.num{text-align:right;font-variant-numeric:tabular-nums}
.good{color:var(--good)}.bad{color:var(--bad)}
.breaks{font-family:var(--font-mono);font-size:.78rem;white-space:pre-wrap;color:var(--muted)}
"""


def png_data_uri(path: Path, cache: dict) -> str:
    """Recompress losslessly with ImageMagick (tiny-skia writes barely compressed PNGs)."""
    if path in cache:
        return cache[path]
    data = path.read_bytes()
    try:
        out = subprocess.run(["magick", str(path), "-strip", "-define", "png:compression-level=9", "PNG:-"],
                             capture_output=True, check=True).stdout
        if out and len(out) < len(data):
            data = out
    except (OSError, subprocess.CalledProcessError):
        pass
    uri = "data:image/png;base64," + base64.b64encode(data).decode()
    cache[path] = uri
    return uri


def scrub(text: str) -> str:
    """Home directories become `~`; the session scratch directory becomes `<scratch>`."""
    text = re.sub(r"/private/tmp/claude-[0-9]+/[^/\s\"']+/[^/\s\"']+/scratchpad", "<scratch>", text)
    return re.sub(r"/(Users|home)/[^/\s\"']+", "~", text)


def build_gallery(spike: Path, host_data: dict, scorecard: dict, newline_rows: list, out: Path):
    mac = host_data["macos"]
    cache = {}
    parts = [
        '<meta charset="utf-8">',
        '<title>Merman Measurement Bake-off</title>',
        '<meta name="claude:viewport-fit-sides" content="cover">',
        '<link rel="preconnect" href="https://fonts.googleapis.com">',
        '<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600&family=IBM+Plex+Sans:wght@400;600&display=swap">',
        f"<style>{CSS}</style>",
        "<main>",
        "<h1>Merman Measurement Bake-off</h1>",
        '<p class="lede">Spike S2 for <code>2026-10-21-merman</code>. Option A shapes labels with rustybuzz (a mirror of usvg 0.47); '
        "option B is Merman's native measurement on the same font files. Both render with Merman "
        f"<code>{REV_SHORT}</code>, the resvg-safe pipeline, Mermaid's dark theme, and resvg 0.47 drawing from exactly those files. "
        "Images below are macOS (Trebuchet MS). "
        '<span class="legend"></span>is each label\'s measured width, drawn over the text resvg actually drew.</p>',
    ]
    # Scorecard (compact).
    parts.append("<h2>Scorecard</h2><div class='scroll'><table><tr><th>Area</th><th>Host</th>"
                 + "".join(f"<th>{html.escape(MEASURER_LABEL[m])}</th>" for m in MEASURERS) + "</tr>")
    for host in ("macos", "linux"):
        sc = scorecard.get(host)
        if not sc:
            continue
        acc = sc["accuracy"]
        parts.append(f"<tr><td>Width vs drawn (max / p95 / labels &gt;1 px)</td><td>{host}</td>"
                     + "".join(f"<td class='num'>{acc[m]['max_px']:.2f} / {acc[m]['p95_px']:.2f} / {acc[m]['above_1px']}</td>" for m in MEASURERS) + "</tr>")
        de = sc["defects"]
        parts.append(f"<tr><td>Clipped / past box / off canvas (worst clip px)</td><td>{host}</td>"
                     + "".join(f"<td class='num'>{de[m]['clipped']} / {de[m]['past_box']} / {de[m]['off_canvas']} ({de[m]['worst_clip_px']:.1f})</td>" for m in MEASURERS) + "</tr>")
        wr = sc["wrapping12_multiline"]
        parts.append(f"<tr><td>Multi-line labels: same breaks as Mermaid.js 12.2.0 / matched</td><td>{host}</td>"
                     + "".join(f"<td class='num'>{wr[m]['same_breaks']} / {wr[m]['matched']}</td>" for m in MEASURERS) + "</tr>")
        fb = sc["fallback_mismatch_labels"]
        parts.append(f"<tr><td>Labels with a face mismatch</td><td>{host}</td>"
                     + "".join(f"<td class='num'>{fb[m]}</td>" for m in MEASURERS) + "</tr>")
        tm = sc.get("timing") or {}
        if tm:
            parts.append(f"<tr><td>Cold render, median of per-diagram medians (ms)</td><td>{host}</td>"
                         + "".join(f"<td class='num'>{tm[m]['cold_median_of_medians_ms']}</td>" for m in MEASURERS) + "</tr>")
            parts.append(f"<tr><td>Warm render, median of per-diagram medians (ms)</td><td>{host}</td>"
                         + "".join(f"<td class='num'>{tm[m]['warm_median_of_medians_ms']}</td>" for m in MEASURERS) + "</tr>")
    parts.append("</table></div>")

    refs12_dir = mac["refs12_dir"]
    wraps = {m: {(x["input"], tuple(x["reference"])): x for x in mac["wrap_labels12"][m]} for m in MEASURERS}
    wraps11 = {(x["input"], " ".join(x["reference"])): x["reference"] for x in mac["wrap_labels"]["b"]}
    for set_, title in (("mixed", "Mixed scripts"), ("wrap", "Wrapping")):
        parts.append(f"<h2>{title}</h2>")
        if set_ == "wrap":
            parts.append("<p>Line breaks per label, against Mermaid.js 12.2.0 (the version Merman models) in the same Chrome. "
                         "Mermaid.js 11.12.2 (the local <code>mmdc</code>) wraps flowchart labels at 200 px instead of 120 px; "
                         "its lines are shown where they differ. Green: same breaks as Mermaid.js 12.2.0.</p>")
        for row in [r for r in mac["rows"] if r["input"]["set"] == set_]:
            name = row["input"]["name"]
            parts.append(f"<section class='case'><header><b>{html.escape(name)}</b>"
                         f"<span class='chip'>{html.escape(row['input']['family'])}</span></header><div class='panels'>")
            for m in ("a", "b"):
                res = row["results"][m]
                src = mac["out_dir"] / res["png"] if res.get("png") else None
                de = (res.get("analysis") or {}).get("defects") or {}
                cap = f"{MEASURER_LABEL[m]}: {de.get('clipped', 0)} clipped, {de.get('backing_narrower', 0)} past box"
                img = f"<img alt='{m} render' src='{png_data_uri(src, cache)}'>" if src and src.exists() else "<p>render failed</p>"
                parts.append(f"<figure>{img}<figcaption>{html.escape(cap)}</figcaption></figure>")
            ref_png = refs12_dir / f"{name}.mermaid-js.png"
            img = f"<img alt='Mermaid.js reference' src='{png_data_uri(ref_png, cache)}'>" if ref_png.exists() else "<p>no reference</p>"
            parts.append(f"<figure>{img}<figcaption>Mermaid.js {html.escape(mac['refs12']['mermaid_version'])} "
                         f"(Chrome, {html.escape(mac['refs12']['font_family'])})</figcaption></figure></div>")
            lines = []
            for (inp, ref_lines), x in wraps["b"].items():
                if inp != name:
                    continue
                a = wraps["a"].get((inp, ref_lines))
                amw = wraps["a-mw"].get((inp, ref_lines))
                if len(ref_lines) < 2 and not any(y and y["ours"] and len(y["ours"]) > 1 for y in (a, amw, x)):
                    continue
                cell = lambda y: ("<span class='bad'>not found</span>" if not y or not y["matched"] else
                                  f"<span class='{'good' if y['same_breaks'] else 'bad'}'>" + html.escape(" ⏎ ".join(y["ours"])) + "</span>")
                old_lines = wraps11.get((inp, " ".join(ref_lines)))
                ref_cell = html.escape(" ⏎ ".join(ref_lines))
                lines.append(f"<tr><td class='breaks'>{ref_cell}</td><td class='breaks'>{cell(a)}</td><td class='breaks'>{cell(amw)}</td><td class='breaks'>{cell(x)}</td></tr>")
            if lines:
                parts.append("<div class='scroll'><table><tr><th>Mermaid.js 12.2.0 lines</th><th>A (greedy)</th><th>A + Merman wrap</th><th>B</th></tr>"
                             + "".join(lines) + "</table></div>")
            parts.append("</section>")

    parts.append("<h2><code>\\n</code> in labels</h2>")
    parts.append("<p>Each label holds a literal backslash-n: <code>Alpha one\\nBravo two is longer</code>. "
                 "“After rewrite” replaces it with <code>&lt;br/&gt;</code> before parsing. "
                 "“Box fits widest line” compares the label's box with the same diagram drawn on one line "
                 "(<code>\\n</code> replaced by a space): the box keeps that one-line padding around the widest line. "
                 "“Box keeps one-line width” is the waste Ken saw in Zed.</p>")
    parts.append("<div class='scroll'><table><tr><th>Family</th><th>Mermaid.js 11.12.2</th><th>Mermaid.js 12.2.0</th>"
                 "<th>Merman (B), as written</th><th>B after rewrite</th><th>A after rewrite</th></tr>")
    for e in newline_rows:
        b_raw, b_rw = newline_verdict(e, "b")
        _, a_rw = newline_verdict(e, "a")
        cells = [e["family"], e["mermaid_js_11"], e["mermaid_js_12"], b_raw, b_rw, a_rw]
        parts.append("<tr>" + "".join(f"<td>{html.escape(str(c))}</td>" for c in cells) + "</tr>")
    parts.append("</table></div>")
    parts.append(f"<p class='lede'>Generated by <code>scripts/report.py</code>; raw data in <code>results.json</code>.</p></main>")
    out.write_text(scrub("\n".join(parts)))


REV_SHORT = "482fcde"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--spike", type=Path, default=Path("."))
    parser.add_argument("--host", nargs=5, action="append", metavar=("OS", "OUT", "REFS", "REFS12", "TIMING"))
    args = parser.parse_args()
    spike = args.spike
    host_data, scorecard, raw = {}, {}, {"hosts": {}}
    for os_, out_dir, refs_dir, refs12_dir, timing_path in args.host:
        out_dir, refs_dir, refs12_dir = Path(out_dir), Path(refs_dir), Path(refs12_dir)
        refs = json.loads((refs_dir / "mermaid-js.json").read_text())
        refs12 = json.loads((refs12_dir / "mermaid-js.json").read_text())
        variants = {}
        for path in sorted(out_dir.glob(f"results-{os_}-*.json")):
            d = json.loads(path.read_text())
            variants[d["variant"]] = d
        unset = variants["unset"]
        rows = unset["rows"]
        timing = json.loads(Path(timing_path).read_text()) if timing_path != "-" and Path(timing_path).exists() else None
        wrap_labels = {m: wrapping(rows, refs, m, {"fixture", "mixed", "wrap"}) for m in MEASURERS}
        wrap_labels12 = {m: wrapping(rows, refs12, m, {"fixture", "mixed", "wrap"}) for m in MEASURERS}
        sc = {
            "accuracy": {m: accuracy(rows, m) for m in MEASURERS},
            "accuracy_by_set": {s: {m: accuracy(rows, m, {s}) for m in MEASURERS} for s in ("fixture", "mixed", "wrap")},
            "accuracy_by_variant": {v: {m: accuracy(d["rows"], m) for m in MEASURERS} for v, d in variants.items()},
            "defects": {m: defects(rows, m) for m in MEASURERS},
            "defects_by_set": {s: {m: defects(rows, m, {s}) for m in MEASURERS} for s in ("fixture", "mixed", "wrap")},
            "defects_by_variant": {v: {m: defects(d["rows"], m) for m in MEASURERS} for v, d in variants.items()},
            "wrapping_all": {m: wrap_summary(wrap_labels[m]) for m in MEASURERS},
            "wrapping_multiline": {m: wrap_summary(wrap_labels[m], multi_only=True) for m in MEASURERS},
            "wrapping_multiline_by_set": {s: {m: wrap_summary([x for x in wrap_labels[m] if x["set"] == s], multi_only=True) for m in MEASURERS} for s in ("fixture", "mixed", "wrap")},
            "wrapping12_all": {m: wrap_summary(wrap_labels12[m]) for m in MEASURERS},
            "wrapping12_multiline": {m: wrap_summary(wrap_labels12[m], multi_only=True) for m in MEASURERS},
            "wrapping12_multiline_by_set": {s: {m: wrap_summary([x for x in wrap_labels12[m] if x["set"] == s], multi_only=True) for m in MEASURERS} for s in ("fixture", "mixed", "wrap")},
            "fallback_mismatch_labels": {m: len(fallback_mismatches(rows, m)) for m in MEASURERS},
            "fallback_mismatches": {m: fallback_mismatches(rows, m) for m in MEASURERS},
            "fallback_mismatches_by_variant": {v: {m: fallback_mismatches(d["rows"], m) for m in MEASURERS} for v, d in variants.items()},
            "drawn_fallback_faces": drawn_fallback_faces(rows),
            "calls": calls_summary(rows),
            "b_declines": sum((r["results"]["b"]["calls"].get("declines") or 0) for r in rows if "b" in r["results"]),
            "b_same_as_production": all(r["results"]["b"].get("same_as_production") in (True, None) for r in rows if "b" in r["results"]),
            "chain": {v: d["chain"] for v, d in variants.items()},
            "native_context_load_ms": {v: round(d["native_context_load_ms"], 1) for v, d in variants.items()},
        }
        if timing:
            sc["timing"], sc["timing_by_diagram"] = timing_summary(timing, rows)
        scorecard[os_] = sc
        nl = newline_table(rows, refs, refs12)
        host_data[os_] = {"rows": rows, "refs": refs, "refs12": refs12, "refs_dir": refs_dir, "refs12_dir": refs12_dir,
                          "out_dir": out_dir, "wrap_labels": wrap_labels, "wrap_labels12": wrap_labels12}
        raw["hosts"][os_] = {
            "scorecard": sc,
            "wrap_labels": wrap_labels,
            "wrap_labels_mermaid12": wrap_labels12,
            "newline": nl,
            "mermaid_js": refs,
            "mermaid_js_12": refs12,
            "variants": {v: d for v, d in variants.items()},
            "timing": timing,
        }
    raw["newline_verdicts"] = {
        os_: [{"family": e["family"], "mermaid_js_11": e["mermaid_js_11"], "mermaid_js_12": e["mermaid_js_12"],
               "b_raw": newline_verdict(e, "b")[0], "b_rewritten": newline_verdict(e, "b")[1],
               "a_raw": newline_verdict(e, "a")[0], "a_rewritten": newline_verdict(e, "a")[1]} for e in d["newline"]]
        for os_, d in raw["hosts"].items()
    }
    (spike / "results.json").write_text(scrub(json.dumps(raw, separators=(",", ":"), ensure_ascii=False, default=str)) + "\n")
    (spike / "gallery").mkdir(exist_ok=True)
    build_gallery(spike, host_data, scorecard, raw["hosts"]["macos"]["newline"], spike / "gallery" / "index.html")
    # Console scorecard.
    for os_, sc in scorecard.items():
        print(f"\n## {os_}")
        print(json.dumps({k: sc[k] for k in ("accuracy", "wrapping_all", "wrapping_multiline", "wrapping_multiline_by_set",
                                             "wrapping12_all", "wrapping12_multiline", "wrapping12_multiline_by_set",
                                             "fallback_mismatch_labels", "calls", "b_declines", "b_same_as_production",
                                             "native_context_load_ms", "drawn_fallback_faces")}, indent=1))
        print(json.dumps({m: {k: v for k, v in sc["defects"][m].items() if k != "examples"} for m in MEASURERS}))
        print(json.dumps(sc.get("timing"), indent=1))
    for os_, verdicts in raw["newline_verdicts"].items():
        print(f"\n## newline {os_}")
        for v in verdicts:
            print(v)


if __name__ == "__main__":
    main()

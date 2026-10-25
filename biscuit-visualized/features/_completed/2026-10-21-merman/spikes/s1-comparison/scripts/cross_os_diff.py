#!/usr/bin/env python3
"""Cross-OS PNG difference (R-S9, AC-S8 input for the D13 tolerance), macOS vs Linux.

For every input, theme, and renderer that produced a PNG on both hosts:
- if the pixel sizes differ, records that (fonts changed the layout);
- otherwise records the share of differing pixels (ImageMagick `AE`, fuzz 0)
  and the normalized RMSE (0..1).

Usage: python3 -I scripts/cross_os_diff.py <macos results.json> <linux results.json> <out.json> [focus input]
"""

import json
import statistics
import subprocess
import sys


def compare(a, b, metric):
    run = subprocess.run(["magick", "compare", "-metric", metric, a, b, "null:"], capture_output=True, text=True)
    text = run.stderr.strip()
    # AE prints "N" or "N (fraction)"; RMSE prints "abs (normalized)".
    if "(" in text:
        absolute, normalized = text.split("(", 1)
        return float(absolute.strip()), float(normalized.rstrip(")").strip())
    return float(text.split()[0]), None


def main():
    mac, linux, out = (json.load(open(sys.argv[1])), json.load(open(sys.argv[2])), sys.argv[3])
    focus = sys.argv[4] if len(sys.argv) > 4 else None
    lrows = {r["input"]["name"]: r for r in linux["rows"]}
    cells = []
    for row in mac["rows"]:
        name = row["input"]["name"]
        other = lrows.get(name)
        if not other:
            continue
        for theme, by_renderer in row["results"].items():
            for renderer, res in by_renderer.items():
                lres = other["results"].get(theme, {}).get(renderer)
                if not (res.get("ok") and lres and lres.get("ok")):
                    continue
                cell = {"input": name, "family": row["input"]["family"], "theme": theme, "renderer": renderer,
                        "macos_px": [res["png_width"], res["png_height"]], "linux_px": [lres["png_width"], lres["png_height"]]}
                if cell["macos_px"] != cell["linux_px"]:
                    cell["same_size"] = False
                else:
                    cell["same_size"] = True
                    ae, _ = compare(res["png"], lres["png"], "AE")
                    _, rmse = compare(res["png"], lres["png"], "RMSE")
                    cell["differing_pixels"] = int(ae)
                    cell["differing_share"] = ae / (res["png_width"] * res["png_height"])
                    cell["rmse"] = rmse
                cells.append(cell)
    summary = {}
    for renderer in ("current", "merman", "merman-real", "merman-native"):
        mine = [c for c in cells if c["renderer"] == renderer]
        same = [c for c in mine if c["same_size"]]
        shares = [c["differing_share"] for c in same]
        summary[renderer] = {
            "compared": len(mine),
            "size_differs": len(mine) - len(same),
            "identical": sum(1 for c in same if c["differing_pixels"] == 0),
            "median_differing_share": statistics.median(shares) if shares else None,
            "max_differing_share": max(shares) if shares else None,
            "median_rmse": statistics.median(c["rmse"] for c in same) if same else None,
        }
    focus_cells = [c for c in cells if c["input"] == focus] if focus else []
    text = "; ".join(
        f"{r}: {s['compared']} PNG pairs, {s['size_differs']} differ in pixel size (font metrics moved the layout), "
        f"{s['identical']} identical; same-size pairs differ in a median {100 * (s['median_differing_share'] or 0):.1f}% "
        f"(max {100 * (s['max_differing_share'] or 0):.1f}%) of pixels, median normalized RMSE {(s['median_rmse'] or 0):.4f}"
        for r, s in summary.items()
    )
    if focus_cells:
        text += ". Focus diagram " + focus + ": " + "; ".join(
            f"{c['theme']}/{c['renderer']} " + (
                f"{100 * c['differing_share']:.2f}% pixels differ, RMSE {c['rmse']:.4f}" if c["same_size"]
                else f"size {c['macos_px'][0]}×{c['macos_px'][1]} vs {c['linux_px'][0]}×{c['linux_px'][1]}")
            for c in focus_cells
        )
    json.dump({"metric": "ImageMagick compare AE (fuzz 0) share of pixels, and normalized RMSE; only same-size pairs",
               "hosts": [mac["host"], linux["host"]], "summary": text, "by_renderer": summary,
               "focus": focus_cells, "cells": cells}, open(out, "w"), indent=1)
    print(text)


if __name__ == "__main__":
    main()

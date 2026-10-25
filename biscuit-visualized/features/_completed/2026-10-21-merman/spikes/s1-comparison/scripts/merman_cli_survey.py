#!/usr/bin/env python3
"""Defect survey of Merman's own export path (merman-cli at rev 482fcde).

For every Merman fixture and gitGraph case, in the dark and default themes:
- `merman-cli render --format svg --svg-pipeline resvg-safe` (the SVG its PNG
  export rasterizes), with Merman's default deterministic text measurement and
  with `--font-measurement native` (measure with real fonts);
- `merman-cli render --format png` (its own PNG, kept for visual checks).
Each SVG is surveyed by the spike binary (`--survey-file … --ci`: the
case-insensitive resolver modeled on Merman's export). The deterministic SVG is
also compared byte-for-byte with the library SVG the spike rendered.

`--font-measurement native` with system discovery fails on macOS
("native font limit system-cmap-bytes exceeded"), so native mode passes explicit
`--font-file` sources (the faces the labels request, if present).

Usage: python3 -I scripts/merman_cli_survey.py <merman-cli> <spike-binary> <spike-dir> <spike-out-dir> <work-dir> <out.json>
"""

import json
import subprocess
import sys
from pathlib import Path


NATIVE_FONT_FILES = [
    "/System/Library/Fonts/Supplemental/Trebuchet MS.ttf",
    "/System/Library/Fonts/Supplemental/Trebuchet MS Bold.ttf",
    "/System/Library/Fonts/Supplemental/Trebuchet MS Italic.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
]


def main() -> None:
    cli, spike_bin, spike, spike_out, work, out = sys.argv[1:7]
    spike, spike_out, work = Path(spike), Path(spike_out), Path(work)
    work.mkdir(parents=True, exist_ok=True)
    configs = {}
    for theme in ("dark", "default"):
        configs[theme] = work / f"{theme}.json"
        configs[theme].write_text(json.dumps({"theme": theme}))
    inputs = []
    manifest = json.loads((spike / "corpus/merman/manifest.json").read_text())
    for e in manifest:
        inputs.append((e["name"], e["family"], spike / "corpus/merman" / e["family"] / f"{e['name']}.mmd"))
    for mmd in sorted((spike / "corpus/cases").glob("*.mmd")):
        inputs.append((mmd.stem, "gitgraph", mmd))

    results = []
    for name, family, mmd in inputs:
        for theme in ("dark", "default"):
            row = {"name": name, "family": family, "theme": theme}
            for mode, extra in (("deterministic", []), ("native", ["--font-measurement", "native", *[a for f in NATIVE_FONT_FILES if Path(f).exists() for a in ("--font-file", f)]])):
                svg = work / f"{name}.{theme}.{mode}.svg"
                run = subprocess.run(
                    [cli, "render", str(mmd), "-q", "--format", "svg", "--svg-pipeline", "resvg-safe",
                     "-c", str(configs[theme]), "-I", "s1", "-o", str(svg), *extra],
                    capture_output=True, text=True,
                )
                if run.returncode != 0 or not svg.exists():
                    row[mode] = {"error": (run.stderr or run.stdout).strip()[-300:]}
                    continue
                survey = subprocess.run([spike_bin, "--survey-file", str(svg), "--ci"], capture_output=True, text=True)
                row[mode] = json.loads(survey.stdout) if survey.returncode == 0 else {"error": survey.stderr[-300:]}
            if theme == "dark":
                png = work / f"{name}.{theme}.png"
                subprocess.run([cli, "render", str(mmd), "-q", "--format", "png", "-c", str(configs[theme]), "-o", str(png)],
                               capture_output=True)
                row["png"] = png.name if png.exists() else None
            library = spike_out / "svg" / theme / "merman" / f"{name}.svg"
            cli_svg = work / f"{name}.{theme}.deterministic.svg"
            if library.exists() and cli_svg.exists():
                row["identical_to_library_svg"] = library.read_bytes() == cli_svg.read_bytes()
            results.append(row)
            print(f"{family:14} {theme:7} {name[:60]}")
    Path(out).write_text(json.dumps(results, indent=1) + "\n")


if __name__ == "__main__":
    main()

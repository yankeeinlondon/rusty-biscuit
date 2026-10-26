#!/usr/bin/env python3
"""Render Mermaid.js references with the local `mmdc` for the inputs that have no
stored Merman reference: the repo's own test diagrams and the gitGraph cases (R-S4).

Writes `<name>.mermaid-js.svg` beside each `.mmd`, and `mmdc-results.json` in
`corpus/` with the mmdc version and any failure text.

Usage: python3 -I scripts/render_mmdc_refs.py <spike-dir>
"""

import json
import subprocess
import sys
from pathlib import Path


def main() -> None:
    corpus = Path(sys.argv[1]) / "corpus"
    version = subprocess.run(["mmdc", "--version"], capture_output=True, text=True).stdout.strip()
    results = {"mmdc_version": version, "theme": "default", "failures": {}}
    for folder in ("tests", "cases"):
        for mmd in sorted((corpus / folder).glob("*.mmd")):
            out = mmd.with_suffix(".mermaid-js.svg")
            run = subprocess.run(
                ["mmdc", "-q", "-t", "default", "-i", str(mmd), "-o", str(out)],
                capture_output=True, text=True, timeout=120,
            )
            # mmdc can exit 0 with Mermaid's "Syntax error in text" diagram.
            error_svg = out.exists() and 'aria-roledescription="error"' in out.read_text()
            if run.returncode != 0 or not out.exists() or error_svg:
                message = (run.stderr or run.stdout).strip()[-600:]
                results["failures"][mmd.stem] = message or "Mermaid.js rendered its syntax-error diagram"
                out.unlink(missing_ok=True)
                print(f"FAIL {mmd.stem}")
    (corpus / "mmdc-results.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"mmdc {version}: {len(results['failures'])} failures")


if __name__ == "__main__":
    main()

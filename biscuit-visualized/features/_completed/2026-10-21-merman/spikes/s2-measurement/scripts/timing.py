#!/usr/bin/env python3
"""Cold and warm timing for spike S2.

Cold: a fresh `s2-measurement cold` process per diagram and measurer, timed
from outside (process start to exit: font discovery, font loading, the native
context for B, render, fix, rasterize, PNG encode). Runs are interleaved
(round r renders every diagram with every measurer before round r+1).

Warm: `s2-measurement warm --seed N` processes; each renders every diagram
once per measurer in its own shuffled order, so no diagram is ever timed twice
in one process.

Usage: python3 -I scripts/timing.py <binary> <analyze-out-dir> <out.json> [--runs 10] [--variant unset]
"""

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def main() -> None:
    binary, analyze_out, out = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    runs = int(sys.argv[sys.argv.index("--runs") + 1]) if "--runs" in sys.argv else 10
    variant = sys.argv[sys.argv.index("--variant") + 1] if "--variant" in sys.argv else "unset"
    spike = Path(__file__).resolve().parent.parent
    inputs = sorted(
        p for set_ in ("merman", "mixed", "wrap") for p in (spike / "corpus" / set_).rglob("*.mmd")
    )
    measurers = ["a", "a-mw", "b"]
    cold: dict[str, dict[str, list[float]]] = {}
    with tempfile.TemporaryDirectory() as tmp:
        png = Path(tmp) / "out.png"
        for r in range(runs):
            for i, mmd in enumerate(inputs):
                order = measurers[(i + r) % 3:] + measurers[:(i + r) % 3]
                for m in order:
                    started = time.perf_counter()
                    done = subprocess.run(
                        [binary, "cold", "--variant", variant, "--measurer", m, "--input", str(mmd),
                         "--out", str(png), "--requested", str(analyze_out)],
                        capture_output=True,
                    )
                    elapsed = (time.perf_counter() - started) * 1000
                    if done.returncode == 0:
                        cold.setdefault(mmd.stem, {}).setdefault(m, []).append(elapsed)
            print(f"cold round {r + 1}/{runs}", file=sys.stderr)
        warm = []
        for seed in range(1, runs + 1):
            path = Path(tmp) / f"warm-{seed}.json"
            subprocess.run([binary, "warm", "--variant", variant, "--seed", str(seed), "--out", str(path),
                            "--requested", str(analyze_out)], check=True, capture_output=True)
            for sample in json.loads(path.read_text()):
                sample["seed"] = seed
                warm.append(sample)
    out.write_text(json.dumps({"runs": runs, "variant": variant, "cold_ms": cold, "warm": warm}, indent=1) + "\n")
    print(f"wrote {out}", file=sys.stderr)


if __name__ == "__main__":
    main()

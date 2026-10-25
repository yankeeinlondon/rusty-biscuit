#!/usr/bin/env bash
# Spike S2, macOS leg end to end. Linux: run the same `analyze` and timing
# commands on the host (see FINDINGS.md, "Reproduce") and pass its output dir,
# references, and timing file as the second --host of report.py.
#
# Usage: scripts/run.sh <scratch-dir> [<mermaid-12.2.0 package dir>] [<linux out> <linux timing>]
set -euo pipefail
scratch=$1
mermaid12=${2:-$scratch/mermaid12/package}
spike=$(cd "$(dirname "$0")/.." && pwd)
export CARGO_TARGET_DIR=$scratch/s2-target
bin=$CARGO_TARGET_DIR/release/s2-measurement
out=$scratch/s2

(cd "$spike" && cargo build --release)
if [[ ! -d $mermaid12 ]]; then
  mkdir -p "$(dirname "$mermaid12")"
  (cd "$(dirname "$mermaid12")" && npm pack mermaid@12.2.0 --silent && tar xzf mermaid-12.2.0.tgz)
fi
inputs=("$spike"/corpus/merman/*/*.mmd "$spike"/corpus/mixed/*.mmd "$spike"/corpus/wrap/*.mmd "$spike"/corpus/newline/*.mmd)
node "$spike/scripts/mermaidjs_ref.mjs" "$out/refs-macos" "Trebuchet MS" -- "${inputs[@]}"
MERMAID_PACKAGE=$mermaid12 node "$spike/scripts/mermaidjs_ref.mjs" "$out/refs12-macos" "Trebuchet MS" -- "${inputs[@]}"
# Linux references use the Linux host's DejaVu Sans, copied into $out/linux-fonts.
if [[ -f $out/linux-fonts/DejaVuSans.ttf ]]; then
  dv="$out/linux-fonts/DejaVuSans.ttf,$out/linux-fonts/DejaVuSans-Bold.ttf"
  node "$spike/scripts/mermaidjs_ref.mjs" "$out/refs-linux" "DejaVu Sans" "$dv" -- "${inputs[@]}"
  MERMAID_PACKAGE=$mermaid12 node "$spike/scripts/mermaidjs_ref.mjs" "$out/refs12-linux" "DejaVu Sans" "$dv" -- "${inputs[@]}"
fi
for variant in unset "installed:Helvetica" "missing:No Such Font"; do
  "$bin" analyze --os macos --out "$out/out-macos" --variant "$variant"
done
python3 -I "$spike/scripts/timing.py" "$bin" "$out/out-macos" "$out/timing-macos.json" --runs 10
hosts=(--host macos "$out/out-macos" "$out/refs-macos" "$out/refs12-macos" "$out/timing-macos.json")
if [[ $# -ge 4 ]]; then
  hosts+=(--host linux "$3" "$out/refs-linux" "$out/refs12-linux" "$4")
fi
python3 -I "$spike/scripts/report.py" --spike "$spike" "${hosts[@]}"
python3 -I "$spike/scripts/findings.py" --spike "$spike"

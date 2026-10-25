#!/usr/bin/env bash
# Reproduce spike S1 on this host: corpus, references, renders, gallery.
#
# Usage: scripts/run.sh <scratch-dir> [linux-results.json]
#   <scratch-dir> receives the build, PNGs, SVGs, and results (keep it outside the repo).
#   A Linux results file (from running the binary with `--os linux` on a Linux host,
#   PNG paths rewritten to local copies) adds index-linux.html and the cross-OS diff.
#
# Needs: cargo, python3, ImageMagick (`magick`), mmdc, and a Merman clone at
# $MERMAN_CLONE (default ~/coding/forks/merman) at rev 482fcde7. Set
# $MERMAN_CLI to a merman-cli built at that rev to add the Merman-export survey.
set -euo pipefail

spike="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mkdir -p "$1" && cd "$1" && pwd)"
linux_results="${2:-}"
merman_clone="${MERMAN_CLONE:-$HOME/coding/forks/merman}"
repo="$(cd "$spike/../../../../.." && pwd)"

python3 -I "$spike/scripts/build_corpus.py" "$merman_clone" "$spike"
python3 -I "$spike/scripts/extract_test_diagrams.py" "$repo" "$spike"
python3 -I "$spike/scripts/render_mmdc_refs.py" "$spike"

export CARGO_TARGET_DIR="$scratch/target"
(cd "$spike" && cargo build --release)
cd "$scratch"
"$CARGO_TARGET_DIR/release/s1-comparison" --os macos --out s1-out/macos --iterations 25
# MERMAID_FONT demo: unset, an installed family, and an uninstalled name.
mkdir -p demo
"$CARGO_TARGET_DIR/release/s1-comparison" --font-demo demo/default.png demo/default.json
MERMAID_FONT=helvetica "$CARGO_TARGET_DIR/release/s1-comparison" --font-demo demo/helvetica.png demo/helvetica.json
MERMAID_FONT="Nonexistent Sans" "$CARGO_TARGET_DIR/release/s1-comparison" --font-demo demo/missing.png demo/missing.json

cross_os=()
if [ -n "$linux_results" ]; then
  python3 -I "$spike/scripts/cross_os_diff.py" s1-out/macos/results-macos.json "$linux_results" \
    s1-out/cross-os.json upstream_cypress_sequencediagram_spec_example_001
  cross_os=(--cross-os s1-out/cross-os.json)
fi
weight=()
[ -f s1-out/build-weight.json ] && weight=(--build-weight s1-out/build-weight.json)

cli=()
if [ -n "${MERMAN_CLI:-}" ]; then
  python3 -I "$spike/scripts/merman_cli_survey.py" "$MERMAN_CLI" "$CARGO_TARGET_DIR/release/s1-comparison" \
    "$spike" s1-out/macos cli-work s1-out/merman-cli-macos.json
  cli=(--merman-cli s1-out/merman-cli-macos.json)
fi
# The gallery's demo panel reads a JSON list of the demo runs (host, png, width,
# and the fields --font-demo prints); assemble it before building, or omit it.
demo=()
[ -f s1-out/font-demo.json ] && demo=(--font-demo s1-out/font-demo.json)
common=(--spike "$spike" --png-cache png-cache "${weight[@]}" "${cross_os[@]}" "${cli[@]}" "${demo[@]}")
python3 -I "$spike/scripts/build_gallery.py" "${common[@]}" --results s1-out/macos/results-macos.json \
  --theme dark --out "$spike/gallery/index.html" --scorecard "$spike/gallery/scorecard.json" \
  ${linux_results:+--other-results "$linux_results"}
python3 -I "$spike/scripts/build_gallery.py" "${common[@]}" --results s1-out/macos/results-macos.json \
  --theme default --out "$spike/gallery/index-light.html" ${linux_results:+--other-results "$linux_results"}
if [ -n "$linux_results" ]; then
  python3 -I "$spike/scripts/build_gallery.py" "${common[@]}" --results "$linux_results" \
    --theme dark --out "$spike/gallery/index-linux.html" --other-results s1-out/macos/results-macos.json
fi
echo "build weight (slow, optional): $spike/scripts/build_weight.sh $scratch/s1-out"

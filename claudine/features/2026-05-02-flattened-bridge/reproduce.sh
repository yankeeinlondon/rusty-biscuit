#!/usr/bin/env bash
#
# Reproduction script for terminal escape code bleed in non-interactive sessions.
#
# Usage:
#   ./reproduce.sh
#
# This script demonstrates that Terminal::color_mode() triggers an OSC 11
# query on every call when stdout is connected to a TTY. In non-interactive
# sessions (stdin piped, stdout TTY), these repeated queries cause OSC
# response sequences to bleed into rendered output as literal characters.
#
# The pattern observed is:
#   ^[]11;rgb:1a1a/1b1b/2626^[
#
# appearing before each tool call icon once the bleed starts.
#
# Environment requirements:
#   - Must be run in a real terminal emulator (iTerm2, WezTerm, Kitty, etc.)
#   - The `script` command must be available (macOS: built-in; Linux: util-linux)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
FEATURE_DIR="${SCRIPT_DIR}"

echo "=== Terminal Escape Code Bleed Reproduction ==="
echo ""
echo "This script demonstrates repeated OSC 11 queries from Terminal::color_mode()."
echo "Run it in a real terminal emulator (not CI) to see the effect."
echo ""

# Build the test program
cd "${REPO_ROOT}"
echo "Building test program..."
cargo build -p biscuit-terminal --example terminal_info --quiet

# Create a temporary script output file
OUTPUT_FILE="${FEATURE_DIR}/reproduce_output.txt"

# Run in a pseudo-TTY using `script` to simulate TTY-connected stdout
echo ""
echo "Running terminal_info example in pseudo-TTY (captures output to ${OUTPUT_FILE})..."
echo ""

if command -v script >/dev/null 2>&1; then
    # macOS script command: script <output> <command>
    # Linux script command: script -c <command> <output>
    if script -q /dev/null echo "test" >/dev/null 2>&1; then
        # macOS style
        script -q "${OUTPUT_FILE}" \
            cargo run -p biscuit-terminal --example terminal_info --quiet 2>/dev/null || true
    else
        # Linux style
        script -q -c "cargo run -p biscuit-terminal --example terminal_info --quiet 2>/dev/null" \
            "${OUTPUT_FILE}" || true
    fi
else
    echo "ERROR: 'script' command not found. Cannot create pseudo-TTY."
    echo "Install util-linux (Linux) or use macOS built-in."
    exit 1
fi

echo ""
echo "=== Output Analysis ==="
echo ""

# Check for OSC 11 query sequences in the output
# The query sequence is: ESC ] 11 ; ? BEL  (\x1b]11;?\x07)
# The response sequence is: ESC ] 11 ; rgb:... ESC \  (\x1b]11;rgb:...\x1b\\)
if [ -f "${OUTPUT_FILE}" ]; then
    QUERY_COUNT=$(grep -c $'\x1b]11;?' "${OUTPUT_FILE}" 2>/dev/null || echo "0")
    RESPONSE_COUNT=$(grep -c $'\x1b]11;rgb:' "${OUTPUT_FILE}" 2>/dev/null || echo "0")

    echo "OSC 11 queries detected in output: ${QUERY_COUNT}"
    echo "OSC 11 responses detected in output: ${RESPONSE_COUNT}"
    echo ""

    if [ "${QUERY_COUNT}" -gt 0 ] || [ "${RESPONSE_COUNT}" -gt 0 ]; then
        echo "REPRODUCED: OSC sequences found in terminal output."
        echo ""
        echo "Sample sequences from output:"
        grep -o $'\x1b]11;[^\x07]*\x07\|\x1b]11;[^\x1b]*\x1b\\\\' "${OUTPUT_FILE}" 2>/dev/null | head -5 || true
        echo ""
        echo "These sequences should NOT appear in rendered output."
        echo "They are internal terminal queries/responses leaking into stdout."
    else
        echo "No OSC sequences detected in output."
        echo "This could mean:"
        echo "  1. The terminal emulator handles OSC queries transparently"
        echo "  2. The example did not trigger color_mode() in this environment"
        echo "  3. The fix has already been applied"
        echo ""
        echo "To verify the underlying issue, check the code paths documented in"
        echo "baseline.md and examine the call chain from Status::to_terminal()"
        echo "to query_osc_actual()."
    fi

    # Also show any raw escape sequences in the output
    ESC_COUNT=$(grep -c $'\x1b' "${OUTPUT_FILE}" 2>/dev/null || echo "0")
    echo ""
    echo "Total escape sequences in output: ${ESC_COUNT}"
else
    echo "ERROR: Output file not created."
    exit 1
fi

echo ""
echo "=== Call Chain Summary ==="
echo ""
echo "The following call chain leads to repeated OSC queries:"
echo ""
echo "  Status::to_terminal()"
echo "    -> Terminal::color_mode()          [static method, calls free function]"
echo "      -> color_mode()                  [free function in detection/color.rs]"
echo "        -> bg_color()                  [calls query_osc_actual(11, timeout)]"
echo "          -> query_osc_actual()        [sends \\x1b]11;?\\x07 to stdout]"
echo ""
echo "This chain executes EVERY TIME a Status component is rendered,"
echo "which happens frequently during claudine's live output rendering."
echo ""
echo "Multiple hot paths trigger this:"
echo "  1. Status::to_terminal()         [biscuit-terminal/lib/src/components/status.rs:509]"
echo "  2. Table::render()               [biscuit-terminal/lib/src/components/table/table.rs:1344]"
echo "  3. MermaidRenderer::for_terminal() [biscuit-terminal/lib/src/components/mermaid.rs:142]"
echo "  4. HorizontalRule::render_image_tier() [biscuit-terminal/lib/src/components/horizontal_rule/mod.rs:344]"
echo "  5. GraphExpression::for_terminal_mode() [biscuit-terminal/lib/src/components/graph_expression.rs:284]"
echo ""
echo "In claudine specifically:"
echo "  wrap_terminal() -> log::terminal() -> Terminal::new()"
echo "  Each Status render in the live stream calls Terminal::color_mode() again"
echo ""

echo "=== Reproduction Complete ==="
echo ""
echo "Output saved to: ${OUTPUT_FILE}"
echo ""
echo "Next steps:"
echo "  1. Review baseline.md for detailed analysis"
echo "  2. Implement Phase 2: Cache color_mode in Terminal struct + process-level OnceLock"
echo "  3. Implement Phase 3: Cache color_mode in darkmatter TerminalOptions"
echo "  4. Implement Phase 4: Set explicit color_mode in claudine non-interactive paths"
echo "  5. Run this script again to verify the fix"

# Unicode With and Shaping Behavior

- [Terminal Unicode Core Specification (GitHub)](https://github.com/contour-terminal/terminal-unicode-core)

## What “Unicode, width, and shaping behavior” really means

Terminal UIs ultimately care about **cells** (columns). The hard problem is mapping:

> **a Unicode string** → **a sequence of terminal cells** → **cursor positions**, wrapping, truncation, alignment, hit-testing, etc.

The “classic” approach is: **per-codepoint width** using `wcwidth()`-style tables. That breaks for modern emoji + modifiers because many things users perceive as *one character* are actually *multiple code points*.

### 1) Grapheme clusters (what users think of as “one character”)

A **grapheme cluster** can be:

- a base letter + combining marks (e.g., `e` + `◌́`)
- emoji + skin tone modifiers
- emoji ZWJ sequences (👩‍🌾 is multiple code points joined by ZWJ)
- regional-indicator pairs for flags

If you treat each code point independently, you can get absurd widths (like 4 or 6 columns) for what should display as a single glyph. That’s exactly the failure Mode 2027 is trying to reduce: terminals should compute width based on **grapheme clusters**, not individual code points (see the motivation and framing in the spec above).

A concrete demonstration of how this shows up as “cursor jumps” is Mitchell Hashimoto’s write-up:

- [Grapheme Clusters and Terminal Emulators (mitchellh.com)](https://mitchellh.com/writing/grapheme-clusters-in-terminals)

### 2) Cell width is not just “1 or 2”

Even after grapheme clustering, terminals have policy choices that affect layout:

- **Emoji presentation selectors (VS15/VS16)**
  VS16 (`U+FE0F`) requests emoji-style presentation; VS15 (`U+FE0E`) requests text presentation. Some stacks treat the rendered width differently depending on selector behavior and font. A good overview in a “terminal correctness” context is Jeff Quast’s Unicode testing write-up:
    - [Terminal Emulators Battle Royale – Unicode Edition! (jeffquast.com)](https://www.jeffquast.com/post/ucs-detect-test-results/)

- **ZWJ sequences**
  A ZWJ (`U+200D`) joins emoji components. If a terminal/library fails to cluster properly, your computed width diverges from what the terminal renders:
    - [Terminal Emulators Battle Royale – Unicode Edition! (jeffquast.com)](https://www.jeffquast.com/post/ucs-detect-test-results/)

- **East Asian Ambiguous Width (“A” width)**
  Certain characters are “Ambiguous” in the Unicode East Asian Width data; some environments treat them as 1 column, some as 2. This is a classic cause of table misalignment across locales.

If you want an automated way to see how a terminal handles these categories, `ucs-detect` is built exactly for this kind of probing:

- [ucs-detect documentation](https://ucs-detect.readthedocs.io/intro.html)

### 3) “Shaping” (fonts, ligatures, and rendering)

Shaping is a separate axis from clustering/width:

- **Ligatures** (`=>`, `===`, etc.) change how glyphs *look* but should still occupy the same number of cells.
- **Font fallback** can cause “tofu” boxes or unexpected appearance differences.
- **Complex scripts** (Arabic/Indic shaping) can have tricky cursor semantics even when cell width is stable.

Most programs can’t fully introspect shaping from a terminal, but they can do two practical things:

1) segment by grapheme clusters for editing/movement
2) compute width using a policy that best matches the terminal

## How Mode 2027 fits in (and what it does / doesn’t solve)

### What Mode 2027 gives you

If Mode 2027 is **supported and enabled**, you can assume the terminal is at least attempting the Terminal Unicode Core model: grapheme clustering drives cell width. The most readable “why this matters” illustration is:

- [Grapheme Clusters and Terminal Emulators (mitchellh.com)](https://mitchellh.com/writing/grapheme-clusters-in-terminals)

You’ll also see Mode 2027 mentioned as something apps/terminals are beginning to coordinate around:

- [What’s New in Neovim 0.11 (gpanders.com)](https://gpanders.com/blog/whats-new-in-neovim-0-11/)

And at least one terminal exposes configuration that interacts with Mode 2027 behavior:

- [ghostty(5) man page (Arch Linux)](https://man.archlinux.org/man/ghostty.5)

### What Mode 2027 does NOT fully guarantee

Mode 2027 is a **binary indicator**. It does not tell you:

- which Unicode version the terminal matches best
- ambiguous-width policy
- which emoji/ZWJ tables the terminal effectively supports
- whether you’re behind tmux/screen rewriting / suppressing queries

Jeff Quast’s survey argues Mode 2027 alone can’t replace feature probing:

- [State of Terminal Emulators in 2025 (jeffquast.com)](https://www.jeffquast.com/post/state-of-terminal-emulation-2025/)

So: Mode 2027 is a strong signal, not a full characterization.

## What a caller of your program needs to “resolve this”

If your Rust program emits metadata, the caller (a TUI lib, CLI framework, formatter, etc.) needs enough information to choose:

1) how to segment user-visible characters
2) how to compute display width
3) how to handle truncation/wrapping/cursor movement consistently

Here’s the concrete decision tree a caller should implement.

### A) Text segmentation (editing, cursor left/right, backspace)

- Always segment into grapheme clusters (Unicode grapheme segmentation) for:
    - cursor movement by “character”
    - deletion/backspace
    - selection

This is independent of terminal support: user expectations are grapheme-based.

What your metadata should tell them:

- `mode_2027: supported/enabled/disabled/unknown` (you already do this)

### B) Display width (layout, alignment, truncation)

Callers need a width function that matches the terminal.

Practical approach:

1. If Mode 2027 is enabled:
   - compute width by grapheme cluster using a Unicode-aware “cluster width” algorithm
   - treat ZWJ sequences and VS16 emoji presentation as a single cluster (often width ≈ 2 on modern terminals, but probe results are better than assumptions)

2. If Mode 2027 is not enabled / unknown:
   - fall back to `wcwidth`-style width behavior, but expect glitches
   - prefer conservative rendering modes for alignment-sensitive output

What your metadata should tell them (beyond Mode 2027):

- `ambiguous_width_policy: narrow|wide|unknown` (locale inference is possible, but probing is more reliable)
- `zwj_support_level` / `vs16_support_level` (best-effort probe results, or “unknown”)
- optionally: `unicode_support_summary` derived from a probe suite (below)

If you want a ready-made probing strategy to model, `ucs-detect` is effectively a reference implementation:

- [ucs-detect documentation](https://ucs-detect.readthedocs.io/intro.html)
- [Example: terminal.exe results](https://ucs-detect.readthedocs.io/sw_results/Terminalexe.html)

### C) Rendering and truncation rules

To keep UI stable, callers should:

- never split a grapheme cluster when slicing strings for display
- truncate by cells, not bytes/chars
- when aligning columns, use the chosen width function consistently
- if terminal behavior is unknown, prefer:
    - ragged-right layouts for “pretty tables”
    - an optional “unicode-safe” mode that avoids alignment-sensitive glyphs

Your metadata can enable this by providing:

- `width_confidence: high|medium|low` derived from Mode 2027 + probes
- `recommended_unicode_profile: unicode_core|legacy_wcwidth|conservative_unknown`

## If you want to make this turnkey for callers

Add a single derived field:

- `unicode_profile`:
    - `unicode_core_grapheme_width` (Mode 2027 enabled or strongly implied)
    - `legacy_wcwidth`
    - `unknown_conservative`

And optionally expose a small “probe suite” result set:

- measured cursor deltas for 3–5 canonical sequences (ZWJ family emoji, VS16 emoji, combining marks)
- timeouts / failures (important behind tmux/screen)

This aligns with the “don’t guess—test” conclusion argued in:

- [State of Terminal Emulators in 2025 (jeffquast.com)](https://www.jeffquast.com/post/state-of-terminal-emulation-2025/)

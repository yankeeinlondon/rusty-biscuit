# Terminal Image Rendering Notes

This document captures the current behavior of inline image rendering in `biscuit-terminal`, why behavior differs by terminal, and what issues remain unresolved.

## Problem Space

Inline terminal images are not standardized across emulators in practice, even when they support the same protocol.

Two commands that look equivalent can still differ in:

- how many rows the terminal considers occupied by the image
- whether the cursor is auto-advanced after image draw
- whether cursor save/restore includes image state side effects
- how floating-point image scaling rounds to terminal cells
- when rendering work is flushed relative to shell prompt redraw

The current debugging baseline command is:

```bash
bt image ../assets/biscuit-terminal.png -w 20 --mb 0
```

The intended result is no blank line below the image when `--mb 0`.

## Terminal Variance Matrix

Current observed behavior from testing in this repo:

| Terminal | Protocol Path | Current Status | Notes |
|---|---|---|---|
| Kitty | Kitty graphics | Correct | No extra trailing blank row for baseline test case |
| Warp | Kitty graphics | Correct | Matches Kitty behavior in current implementation |
| WezTerm | Kitty graphics (`c=` + `r=`) | Partial | Aspect ratio currently stable; still shows a trailing blank line |
| iTerm2 | Native OSC 1337 | Partial | Still shows a trailing blank line; can also appear one-row-sensitive depending sizing |

Important nuance: terminals can render the image pixels correctly but disagree about the logical cursor row after rendering. Most of the remaining bug is cursor row accounting, not image decoding.

## Why `clear` Changes Behavior

`clear` is simple from a user perspective, but terminal state after `clear` is not fully uniform across emulators.

Common reasons image behavior shifts after `clear`:

- `clear` usually emits `CSI H` + `CSI 2J`, which resets cursor position and clears visible content, but not every emulator handles image layers and scrollback surfaces the same way.
- Some terminals treat image planes separately from text cells and recompute clipping/placement after a full-screen clear.
- The shell prompt redraw immediately after `clear` can race with asynchronous image decode/display, changing where the terminal believes the current row is.
- Line-wrap and margin state interactions (`DECAWM`, scroll region behavior) can differ after a hard repaint cycle.
- If a terminal internally rounds image-to-cell height differently at top-of-screen versus scrolled regions, row accounting can change after `clear`.

In short: `clear` can expose state transition bugs in terminal emulators and in applications that rely on exact row math.

## Current Implementation

The image pipeline currently does the following.

1. Detect terminal capabilities and app identity.
2. Select protocol path.
3. Compute expected image height in terminal rows.
4. Emit image sequence wrapped in save/restore cursor controls.
5. Advance cursor by a terminal-specific row correction.
6. Apply explicit top/bottom margins from layout flags.

### Protocol selection

- iTerm2 is forced to native OSC 1337 rendering even if Kitty support is advertised.
- WezTerm uses Kitty with explicit `c=` and `r=` to avoid historical aspect-ratio drift.
- Kitty/Warp use Kitty width-only (`c=`), letting terminal-side aspect handling determine rows.

Primary implementation points:

- `biscuit-terminal/lib/src/components/terminal_image.rs`
- `render_to_terminal()`
- `render_kitty_for_terminal()`
- `render_iterm2_for_terminal()`

### Cursor strategy

Current render sequence shape is:

```text
ESC[s <optional horizontal offset> <image protocol sequence> ESC[u ESC[{rows}B CR
```

This means:

- image drawing does not permanently move the cursor while the image sequence is emitted
- final cursor placement is centralized via explicit row movement
- carriage return normalizes to column 0 for subsequent prompt/text rendering

### Row calculation

Rows are derived from:

- image aspect ratio
- resolved width in terminal cells
- detected cell pixel size from `discovery::fonts::cell_size()`
- fallback cell size `8x16` if detection fails

### CLI emission behavior

`bt` prints the image sequence directly and flushes stdout immediately, without adding extra newline/up-cursor compensation. Margin lines are emitted explicitly via layout options.

Primary CLI point:

- `biscuit-terminal/cli/src/main.rs`
- `emit_image_output()`

## What This Design Solves

- avoids the inverted `%` prompt artifact caused by some prior newline/cursor rewrites
- keeps Kitty and Warp behavior stable for the baseline test case
- keeps WezTerm aspect ratio stable by using explicit row sizing in Kitty protocol
- keeps iTerm2 on its native protocol path for better compatibility than Kitty fallback
- keeps margin behavior explicit and predictable (`--mb` controls intentional blank lines)

## Remaining Issues

The current unresolved issues are:

- iTerm2 still leaves one apparent blank line after image output at `--mb 0`.
- WezTerm still leaves one apparent blank line after image output at `--mb 0`.
- Behavior can still shift after `clear` in some terminals, indicating terminal state transitions are affecting cursor/image accounting.

These are likely due to terminal-internal interpretation differences of logical row occupation versus drawn pixel height, not a single escaping bug.

## Why This Is Hard to Fully Normalize

Terminal image protocols provide transport and sizing knobs, but not a single cross-emulator contract for final cursor row semantics. Even with identical escape sequences, emulators can differ in:

- rounding policy from pixels to rows
- whether row occupancy includes partially filled final rows
- when cursor state is updated relative to image display completion
- how save/restore interacts with non-text rendering operations

That makes perfect parity a calibration problem across terminal implementations.

## Recommended Next Work

If we continue refining this, the most practical path is:

1. Add emulator-specific golden tests using recorded escape output plus screenshot-based assertions in CI (where possible).
2. Introduce an internal per-emulator cursor calibration table keyed by app version ranges.
3. Add a debug mode that prints computed row math and chosen correction for quick field diagnosis.
4. Consider a user override flag to force cursor row correction (`--image-row-adjust`) for edge environments.


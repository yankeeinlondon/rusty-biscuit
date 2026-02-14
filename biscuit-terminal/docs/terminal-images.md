# Terminal Image Rendering Notes

This document captures the current behavior of inline image rendering in `biscuit-terminal`, why behavior differs by terminal, and what was required to achieve consistent rendering across all five target terminals.

## Problem Space

Inline terminal images are not standardized across emulators in practice, even when they support the same protocol.

Two commands that look equivalent can still differ in:

- how many rows the terminal considers occupied by the image
- whether the cursor is auto-advanced after image draw
- whether cursor save/restore includes image state side effects
- how floating-point image scaling rounds to terminal cells
- when rendering work is flushed relative to shell prompt redraw
- how `\x1b[NB` (CUD) interacts with the bottom margin during scroll events
- whether DSR responses arrive cleanly or are contaminated by prior protocol responses

The current debugging baseline command is:

```bash
bt image ../assets/biscuit-terminal.png -w 20 --mb 0
```

The intended result is no blank line below the image when `--mb 0`.

## Terminal Variance Matrix

Current observed behavior from testing in this repo (all terminals verified):

| Terminal | Protocol Path | No-Scroll Status | Scroll Status | Cursor Rounding | Notes |
|---|---|---|---|---|---|
| Kitty | Kitty graphics (width-only) | Correct | Correct | `ceil` | 8x16px cells; scroll compensation via `\n` |
| Ghostty | Kitty graphics (width-only) | Correct | Correct | `ceil` | 16x35px cells; scroll resolved natively by save/restore |
| WezTerm | Kitty graphics (`c=` + `r=`) | Correct | Correct | `ceil` | 16x35px cells (unstable detection; see below); scroll compensation via `\n` |
| Warp | Kitty graphics (width-only) | Correct | N/A | `floor` | 9x19px cells; input always at bottom, images never trigger scroll |
| iTerm2 | Native OSC 1337 | Correct | Correct | `ceil` | 7x17px cells; fixed from `floor` (same diagnosis as Ghostty); scroll compensation via `\n` |

Important nuance: terminals can render the image pixels correctly but disagree about the logical cursor row after rendering. Most of the debugging effort was cursor row accounting, not image decoding.

## Ghostty Diagnosis and Fix

Ghostty previously showed intermittent behavior: correct after `clear`, then a persistent blank line appearing after several renders. DSR diagnostics (`\x1b[6n` cursor position queries) revealed the root cause.

### What the DSR data showed

**No-scroll case** (cursor near top of screen, image fits without scrolling):

```
cursor BEFORE: row=60 col=1
predicted:     image extends to row 72 (screen has 74)
predicted:     no scroll needed
cursor AFTER:  row=71 col=1
actual delta:  11 rows
match:         cursor advanced exactly as expected
```

CUD(11) worked correctly. But with `floor(11.71) = 11`, the cursor landed ON the image's last row — text rendered there was hidden behind the image overlay. It looked "correct" only because the hidden text was not visible.

**Scroll case** (cursor at bottom of screen, image extends past viewport):

```
cursor BEFORE: row=74 col=1
predicted:     image extends to row 86 (screen has 74)
predicted:     SCROLL needed (12 rows past bottom)
cursor AFTER:  row=74 col=1
actual delta:  0 rows
MISMATCH:      expected 11 rows, got 0 (off by -11)
```

The terminal scrolled to fit the image. After scroll, `\x1b[u` (restore) returned cursor to row 74 (the bottom — save/restore uses screen coordinates). CUD(11) was **clamped at the bottom margin** and had zero effect. The cursor stayed at row 74 while the image bottom was at row ~73, creating a 1-row blank line gap.

### Root cause

Two compounding issues:

1. **`floor` rounding was wrong for Ghostty.** An image needing 11.71 rows physically occupies 12 rows on screen (you cannot display 0.71 of a row). Using `floor(11)` placed the cursor ON the last image row, not below it.
2. **CUD is clamped at the bottom margin.** When the cursor is at the screen bottom after a scroll event, `\x1b[NB` cannot advance further — it simply does nothing. The scroll amount (`ceil`) exceeds the floor-based CUD value, so the cursor never reaches the right position.

### Fix

Switched Ghostty from `floor` to `ceil` rounding. With `ceil(11.71) = 12`:

- **No-scroll case**: CUD(12) advances past the full image. No hidden text.
- **Scroll case**: Terminal scrolls by exactly `ceil` rows. Image occupies `ceil` rows at the bottom. Cursor at screen bottom is exactly 1 row past the image. No gap.

Ghostty is unique among tested terminals: its save/restore interaction with scroll naturally positions the cursor correctly when `ceil` rounding is used. No additional scroll compensation is needed.

### Why previous `ceil` testing was wrong

The original observation that "ceil causes a blank line on Ghostty" was made without controlling for scroll state. Testing was done in sessions where scroll had already occurred, so the observed blank line was from the scroll/CUD-clamping bug, not from ceil rounding itself. After a fresh `clear` (no scroll possible), ceil produces correct results.

## WezTerm Diagnosis

WezTerm was tested systematically with `--debug` across three scenarios: post-`clear`, subsequent no-scroll, and scroll.

### What the DSR data showed

**Run 1: Post-`clear`, no scroll** (cell size detection succeeds):

```
cell size:    16x35 px
image height: 11.71 raw → ceil=12 floor=11
cursor rows:  12 (used for CUD)
cursor BEFORE: row=4 col=1
predicted:    image extends to row 16 (screen has 52)
predicted:    no scroll needed
cursor AFTER: row=16 col=1
actual delta: 12 rows
match:        cursor advanced exactly as expected
```

Debug text appeared immediately after the image. ✅

**Run 2: No scroll, cell size detection fails**:

```
cell size:    8x16 px          ← FALLBACK (was 16x35 on run 1)
image height: 12.81 raw → ceil=13 floor=12
cursor rows:  13 (used for CUD)
cursor BEFORE: row=33 col=1
predicted:    image extends to row 46 (screen has 52)
predicted:    no scroll needed
cursor AFTER: (query failed)   ← DSR response lost
```

Debug text appeared correctly despite wrong cell size (CUD=13 instead of correct 12 — the extra row was not visually obvious). ✅

**Run 3: Scroll case** (before scroll compensation fix):

```
cell size:    8x16 px          ← FALLBACK
image height: 12.81 raw → ceil=13 floor=12
cursor rows:  13 (used for CUD)
cursor BEFORE: row=52 col=1
predicted:    image extends to row 65 (screen has 52)
predicted:    SCROLL needed (13 rows past bottom)
cursor AFTER: (query failed)
```

Debug text overlapped the image's last row. ❌ (Now fixed by scroll compensation.)

### Root cause

WezTerm exhibits **the same CUD-clamping mechanism** as Ghostty's original scroll bug: after a scroll event, `\x1b[u` restores the cursor to the screen-relative bottom row, and `\x1b[NB` (CUD) is clamped at the bottom margin with zero effect. The cursor stays on the image's last row.

Unlike Ghostty, `ceil` alone does NOT fix WezTerm's scroll case. Ghostty's save/restore interaction with scroll leaves the cursor exactly 1 row past the image; WezTerm's save/restore does not adjust for scroll in the same way.

### Fix: scroll compensation

The scroll case is resolved by detecting when the image will extend past the viewport and appending a single `\n` after the CUD sequence. See [Scroll Compensation](#scroll-compensation) for details.

### Cell size detection instability

A secondary finding: WezTerm's `cell_size()` detection is **unstable across renders**. The first render after `clear` detects 16x35px correctly, but subsequent renders fall back to 8x16px. This is likely because the Kitty graphics protocol response from the first render contaminates the CSI 16t / CSI 14t cell-size query buffer. The `cursor AFTER: (query failed)` on runs 2 and 3 (DSR response lost) confirms WezTerm has a **response ordering/timing issue** where protocol responses from previous operations interfere with subsequent escape sequence queries.

This means:
- The cell size detection produces different raw_height values between runs (11.71 vs 12.81)
- The correct cell size is 16x35px (matches run 1 after `clear`)
- The 8x16px fallback produces an incorrect but functionally similar CUD value in no-scroll cases

This does not cause visible regressions because both cell sizes produce working `ceil` values, and the scroll compensation handles the scroll case regardless of cell size.

## iTerm2 Diagnosis and Fix

iTerm2 was initially assigned `floor` rounding for its native OSC 1337 protocol. Testing with `--debug` revealed the same root cause as Ghostty.

### What the DSR data showed

**No-scroll case** (with `floor` rounding):

```
cell size:    7x17 px
image height: 10.55 raw → ceil=11 floor=10
cursor rows:  10 (used for CUD)
cursor BEFORE: row=4 col=1
predicted:    image extends to row 15 (screen has 42)
predicted:    no scroll needed
cursor AFTER: row=14 col=1
actual delta: 10 rows
match:        cursor advanced exactly as expected
```

Despite the "match" report, debug text overlapped the image — the cursor advanced 10 rows (floor) but the image physically occupies 11 rows (ceil). The cursor was ON the image's last row, identical to the Ghostty floor diagnosis.

### Fix

Switched iTerm2 from `floor` to `ceil` rounding. The image occupies `ceil` rows regardless of the protocol (Kitty or OSC 1337), so `ceil` is universally correct for row advancement.

Scroll compensation also applies to iTerm2 for the same CUD-clamping reason as Kitty and WezTerm.

## Kitty Diagnosis

Kitty was the original reference baseline and worked correctly for no-scroll cases from the start (8x16px cells, `ceil` rounding). However, testing revealed the same CUD-clamping scroll bug as WezTerm: when the image triggers a scroll event, the cursor is clamped at the bottom margin and subsequent text overlaps the image.

The scroll compensation fix resolves this. Kitty now works correctly in both no-scroll and scroll cases.

## Warp Behavior

Warp uses floor-based row counting — `ceil` overshoots by one row. Warp's input is always rendered at the bottom of the terminal, so images always render from the top of the viewport. In testing, cursor position was always row 1, meaning images never trigger scroll events. Scroll compensation is excluded for Warp.

Warp sets `TERM_PROGRAM=WarpTerminal` (not `"warp"` or `"Warp"`). Previously this caused Warp to fall through to `Other("xterm-256color")` and use the default `ceil` path. Fixed by adding `"WarpTerminal"` to all detection points:

- `get_terminal_app()` — TERM_PROGRAM matching
- `image_support_from_known_terminals()` — known Kitty-capable terminals list
- `image_support_from_env()` — environment heuristic fallback
- `osc8_link_support()` — hyperlink support detection

## Why `clear` Fixed Ghostty

`clear` (`CSI H` + `CSI 2J`) resets the cursor to row 1 at the top of the screen. With the cursor at the top, subsequent image renders fit entirely within the viewport without triggering a scroll. Since the scroll event is what causes CUD clamping, avoiding it makes any rounding strategy appear to work.

Once enough images are rendered to push the cursor near the bottom of the screen, the next image triggers a scroll. With `floor` rounding, the CUD-clamping bug manifests. With `ceil` rounding, the scroll amount matches the image height, so no gap appears.

## Current Implementation

The image pipeline does the following:

1. Detect terminal capabilities and app identity.
2. Select protocol path.
3. Compute expected image height in terminal rows.
4. Emit image sequence wrapped in save/restore cursor controls.
5. Advance cursor by a terminal-specific row correction (CUD).
6. Detect scroll condition and append `\n` compensation if needed.
7. Apply explicit top/bottom margins from layout flags.

### Protocol selection

- iTerm2 is forced to native OSC 1337 rendering even if Kitty support is advertised.
- Most Kitty-protocol terminals (Kitty, Warp, Ghostty) use width-only sizing (`c=` without `r=`), letting the terminal determine row count from the aspect ratio.
- WezTerm requires explicit row sizing (`c=` + `r=`) — width-only causes massively incorrect aspect ratio.

Primary implementation points:

- `biscuit-terminal/lib/src/components/terminal_image.rs`
- `render_to_terminal()`
- `render_kitty_for_terminal()`
- `render_iterm2_for_terminal()`

### Cursor strategy

Two distinct strategies based on protocol:

**Kitty protocol** (Kitty, Warp, WezTerm, Ghostty):

```text
ESC[s <optional horizontal offset> <image protocol sequence> ESC[u ESC[{rows}B CR [optional \n]
```

Save/restore cursor wraps the image sequence. The Kitty protocol does not move the cursor, so explicit row advancement is required.

**iTerm2 protocol** (OSC 1337):

```text
ESC[s <optional horizontal offset> <image protocol sequence> ESC[u ESC[{rows}B CR [optional \n]
```

Same save/restore pattern as Kitty. The OSC 1337 protocol auto-advances the cursor after the image, but save/restore neutralizes that auto-advance so explicit cursor management can take over.

This means:

- save/restore wraps the image to reset cursor after iTerm2's protocol auto-advance
- final cursor placement is centralized via explicit row movement
- carriage return normalizes to column 0 for subsequent prompt/text rendering
- optional `\n` compensates for CUD clamping when scroll is detected

### Row calculation

Rows are derived from:

- image aspect ratio
- resolved width in terminal cells
- detected cell pixel size from `discovery::fonts::cell_size()`
- fallback cell size `8x16` if detection fails

#### Terminal-specific rounding

Almost all terminals use ceil-based row counting — a partially filled final row still consumes a full terminal row. Cursor advancement uses `ceil(raw_height)`.

The one exception is **Warp**, which uses floor-based row counting — ceil overshoots by one row, producing a blank line. Cursor advancement uses `floor(raw_height)`.

For images whose height is an exact integer number of cells, ceil and floor produce the same value, so all terminals agree.

### Scroll compensation

When an image triggers a scroll event (cursor near the bottom of the screen, image extends past the viewport), the `\x1b[u` (restore) + `\x1b[NB` (CUD) sequence fails because CUD is clamped at the bottom margin. The cursor stays on the image's last row instead of below it.

The fix detects this condition before rendering:

1. Query cursor position via DSR (`\x1b[6n`)
2. If `cursor_row + image_rows > terminal_height`, scroll will occur
3. Append a single `\n` after the CUD+CR sequence

This works because `\n` (line feed) at the bottom margin triggers a scroll-up, unlike CUD which is silently clamped. The `\n` pushes the cursor exactly one row past the image.

**Terminal-specific behavior:**

- **Kitty, WezTerm, iTerm2**: Scroll compensation applied when scroll is detected.
- **Ghostty**: Excluded — its save/restore interaction with scroll naturally positions the cursor correctly with `ceil` rounding. Adding `\n` would create a blank line.
- **Warp**: Excluded — its input is always at the bottom, so images render at the top and never trigger scroll.

### CLI emission behavior

`bt` prints the image sequence directly and flushes stdout immediately, without adding extra newline/up-cursor compensation. Margin lines are emitted explicitly via layout options.

Primary CLI point:

- `biscuit-terminal/cli/src/main.rs`
- `emit_image_output()`

## Debugging with `--debug`

The `bt image` command supports a `--debug` flag that prints cursor position diagnostics to stderr:

```bash
bt image ../assets/biscuit-terminal.png -w 20 --mb 0 --debug
```

Output includes:

- **terminal dimensions** (cols x rows)
- **cell size** in pixels (from `discovery::fonts::cell_size()`)
- **image width** in cells
- **image height** — raw float value, `ceil`, and `floor`
- **app** — detected terminal application
- **cursor rows** — the CUD value actually used
- **cursor BEFORE** — row and column via DSR query before image render
- **predicted scroll** — whether the image extends past the bottom of the screen
- **cursor AFTER** — row and column via DSR query after image render
- **actual delta** — how many rows the cursor actually advanced
- **MISMATCH** — flagged when actual delta differs from expected

The DSR query uses `\x1b[6n` (Device Status Report) and parses the `\x1b[{row};{col}R` response. Implementation is in `biscuit-terminal/lib/src/discovery/cursor_position.rs`.

### How to use the debug flag

Run the same command repeatedly (starting from `clear`) until behavior changes. Compare the diagnostic output between correct and incorrect renders:

```bash
clear
bt image ../assets/biscuit-terminal.png -w 20 --mb 0 --debug
# repeat until scroll case triggers
```

Key signals:

- **"no scroll needed"** → CUD operates normally
- **"SCROLL needed"** → scroll compensation activates (except Ghostty/Warp)
- **"MISMATCH"** → cursor did not advance as expected, indicating a regression
- **"(query failed)"** → DSR response lost, likely due to response contamination from prior protocol sequences

### Known DSR limitations

WezTerm's DSR responses become unreliable after the first image render in a session. The `cursor AFTER` query fails on run 2+ because Kitty graphics protocol responses from the previous render contaminate the terminal's response buffer. The `cursor BEFORE` query continues to work because enough time elapses between renders (the shell prompt redraw acts as a natural buffer flush).

This does not affect image rendering or scroll compensation — only the debug diagnostics. Cell size detection is also affected (see WezTerm Diagnosis above).

## What This Design Solves

- avoids the inverted `%` prompt artifact caused by some prior newline/cursor rewrites
- keeps all five target terminals rendering consistently in both no-scroll and scroll cases
- keeps WezTerm aspect ratio stable by using explicit row sizing in Kitty protocol
- keeps iTerm2 on its native protocol path for better compatibility than Kitty fallback
- keeps margin behavior explicit and predictable (`--mb` controls intentional blank lines)
- handles scroll-induced CUD clamping transparently via `\n` compensation

## Terminal-Specific Testing Status

All terminals verified as of February 2026:

| Terminal | No-Scroll | Scroll | Cell Size | Rounding | Scroll Compensation | Verified |
|----------|-----------|--------|-----------|----------|---------------------|----------|
| Kitty | ✅ Correct | ✅ Correct | 8x16 (stable) | `ceil` | Yes (when scroll detected) | Yes |
| Ghostty | ✅ Correct | ✅ Correct | 16x35 (stable) | `ceil` | No (handled natively) | Yes |
| WezTerm | ✅ Correct | ✅ Correct | 16x35 (unstable) | `ceil` | Yes (when scroll detected) | Yes |
| Warp | ✅ Correct | N/A | 9x19 | `floor` | No (never scrolls) | Yes |
| iTerm2 | ✅ Correct | ✅ Correct | 7x17 | `ceil` | Yes (when scroll detected) | Yes |

## Remaining Issues

### Cell size detection instability (WezTerm)

WezTerm's cell size detection succeeds on the first query after `clear` (16x35px) but falls back to 8x16px on subsequent queries. This is caused by Kitty graphics protocol responses contaminating the escape sequence response buffer. The DSR (cursor position) query also fails on subsequent runs.

This does not cause visible regressions because:

- 16x35 gives raw=11.71, ceil=12
- 8x16 fallback gives raw=12.81, ceil=13
- Both work correctly in the no-scroll case (CUD advances past the image either way)
- Scroll compensation handles the scroll case regardless of cell size

However, it could cause issues for images where the 1-row CUD difference matters. A potential fix would be to add a response buffer flush (read and discard pending responses) before querying cell size.

### DSR reliability (WezTerm, Kitty)

The `cursor AFTER` DSR query fails on some terminals after image rendering. This only affects `--debug` diagnostics, not rendering. The `cursor BEFORE` query works reliably because the shell prompt redraw between commands acts as a natural buffer flush.

### Attempted approaches (historical)

| Approach | Ghostty | WezTerm | iTerm2 | Kitty | Warp |
|----------|---------|---------|--------|-------|------|
| `ceil` + scroll compensation | ✅ both cases | ✅ both cases | ✅ both cases | ✅ both cases | N/A |
| `ceil` cursor advance (no compensation) | ✅ both cases | ✅ no-scroll / ❌ scroll | ✅ no-scroll / ❌ scroll | ✅ no-scroll / ❌ scroll | ❌ blank line |
| `floor` cursor advance | Intermittent blank line | Text overlap (both cases) | Text overlap (both cases) | ❌ scroll overlap | ✅ both cases |
| `ceil + 1` cursor advance | Not tested | ❌ blank line / ❌ overlap | Not tested | Not tested | Not tested |
| `floor - 1` cursor advance | N/A | Crops image; overlap | Text overlap (too few rows) | N/A | N/A |
| Remove `r=` for WezTerm | N/A | Massively wrong aspect ratio | N/A | N/A | N/A |
| Remove save/restore for iTerm2 | N/A | N/A | Inverted `%` + two blank lines | N/A | N/A |
| `\n` repeated N times instead of CUD | Not tested | Massive over-scroll | Not tested | Not tested | Not tested |
| Remove save/restore selectively | Inconsistent | Inconsistent | N/A | N/A | N/A |
| Kitty `C=1` native cursor | No scroll; cursor behind image | Nothing renders | N/A | N/A | N/A |
| Pad image to exact cell boundary | Always blank line | No change | N/A | N/A | N/A |
| Delete all placements before render | Still intermittent; destroys images | N/A | N/A | N/A | N/A |



## What NOT to Do

These approaches have been tested and caused regressions. Do not re-attempt them without strong new evidence.

### Do NOT replace CUD with newlines for primary cursor advancement

Replacing `\x1b[NB` (CUD) with `\n` repeated N times for cursor advancement causes catastrophic over-scrolling. In WezTerm, the prompt was pushed to the very bottom of the screen with a massive blank gap between image and prompt. The theory was sound (newlines scroll at the bottom margin while CUD does not), but in practice each `\n` triggers both a line feed AND a carriage return via the terminal driver's ONLCR translation, and the interaction with save/restore cursor wrapping produces far more scroll than intended. The terminal's own image rendering has already handled any necessary scrolling; adding scroll-capable advancement on top double-counts it.

Note: a *single* `\n` appended after CUD+CR specifically for the scroll case is different — it compensates for exactly 1 row of CUD clamping. The failed approach was using `\n` *instead of* CUD for all N rows.

### Do NOT remove save/restore cursor wrapping

Removing `\x1b[s`/`\x1b[u` around the image sequence causes regressions on every terminal tested:

- **iTerm2**: Inverted `%` prompt artifact (zsh no-newline indicator) and two blank lines.
- **WezTerm and Ghostty** (Kitty protocol only, targeted removal): Inconsistent behavior — sometimes many blank lines, sometimes just one. Worse than the stable single blank line from the baseline. The theory that save/restore captures a stale pre-scroll position and that Kitty protocol's no-cursor-move makes it a safe no-op was wrong in practice. Terminals appear to rely on the save/restore sequence as part of their image placement bookkeeping, even for Kitty protocol.

Save/restore is required for **all** terminals and **all** protocols. Do not remove it selectively or universally.

### Do NOT remove `r=` from WezTerm's Kitty protocol

WezTerm requires both `c=` (columns) and `r=` (rows) in the Kitty graphics protocol. Using width-only (`c=` without `r=`) causes massively incorrect aspect ratios. Other Kitty-protocol terminals (Kitty, Warp, Ghostty) work correctly with width-only.

### Do NOT use Kitty `C=1` for native cursor movement

The Kitty graphics protocol specifies `C=1` to let the terminal move the cursor to column 0 of the row after the image. In theory this delegates all cursor math to the terminal. In practice:

- **WezTerm**: Does not render the image at all when `C=1` is present. Likely treats the unrecognized parameter as an error and silently discards the entire graphics command.
- **Ghostty**: Renders the image but does not scroll the terminal to accommodate it. The image extends below the viewport and the cursor lands behind the image at its starting position. The terminal recognizes `C=1` but does not implement the scroll-to-fit behavior needed for it to work.

`C=1` may become viable if WezTerm and Ghostty improve their implementations, but as of February 2026, it breaks more than it fixes.

### Do NOT use `floor` rounding for Ghostty or iTerm2

DSR diagnostics confirmed that images physically occupy `ceil` rows on both Ghostty and iTerm2 (and all other tested terminals except Warp). Using `floor` places the cursor ON the image's last row, hiding text behind the image overlay. The apparent "correct" rendering with `floor` was the overlap being invisible. When a scroll event occurs, the `floor` CUD value is clamped at the bottom margin while the `ceil`-based scroll creates a gap.

### Do NOT use `floor - 1` cursor advance for WezTerm

Using `floor(raw_height) - 1` for WezTerm crops the bottom of the image (the last row is visually cut off) while the blank line persists. The reduced cursor advance causes the save/restore + CUD sequence to position one row higher, clipping the image, but the blank line has a separate cause that subtracting rows cannot address.

### Do NOT use `ceil + 1` cursor advance for WezTerm

Using `ceil(raw_height) + 1` produces a blank line in the no-scroll case (1 row too many) and still produces overlap in the scroll case (CUD clamped regardless). This confirms the scroll case cannot be solved by adjusting CUD alone — the issue is that `\x1b[u` restores to the screen-relative bottom, and any CUD value is clamped to 0 at the bottom margin.

### Do NOT pad images to exact cell boundaries

Padding the image canvas with transparent pixels at the bottom to eliminate fractional cell rows (making `ceil == floor`) had no effect on WezTerm's blank line and made Ghostty strictly worse — it lost its post-`clear` correct behavior and now always shows a blank line. The padding changes the image's aspect ratio, which causes terminals to recalculate row occupancy. This confirms the blank line is not caused by ceil/floor disagreement — the root cause is elsewhere in the terminal's cursor/scroll bookkeeping.

### Do NOT delete all Kitty graphics placements before each render

Prepending `\x1b_Ga=d,d=A,q=2\x1b\\` (Kitty protocol "delete all images") before each image render was tested as a way to replicate the state reset that `clear` provides. In practice:

- **All other images on screen are destroyed** — any previously rendered images disappear when a new image is rendered. This is a major regression for any workflow that displays multiple images.
- **Does not fix the intermittent blank line on Ghostty.** The same pattern persists: correct rendering for a variable number of renders, then a persistent blank line appears. This disproves the hypothesis that accumulated placements are the root cause.
- **The `clear` reset mechanism is not about image placements.** The `clear` command resets cursor to the top of the screen, which eliminates the scroll condition that triggers CUD clamping. It has nothing to do with the Kitty graphics placement cache.

### General principles

- **Always test in all 5 target terminals** (Kitty, Warp, WezTerm, Ghostty, iTerm2) before declaring a fix.
- **Never assume a mechanism that works for one terminal works for all.** The variance matrix exists because terminals genuinely disagree on cursor semantics.
- **Regressions on working terminals are worse than unsolved cosmetic issues.** A full-screen gap or broken prompt is a showstopper.
- **Use `bt image --debug` to diagnose new issues.** DSR cursor position queries before and after rendering reveal exactly what the terminal is doing.
- **Always test both no-scroll AND scroll cases.** Run after `clear` (no-scroll) then repeat until the cursor is near the bottom of the screen (scroll). These are mechanically different code paths in the terminal.

## Why This Was Hard to Normalize

Terminal image protocols provide transport and sizing knobs, but not a single cross-emulator contract for final cursor row semantics. Even with identical escape sequences, emulators can differ in:

- rounding policy from pixels to rows
- whether row occupancy includes partially filled final rows
- when cursor state is updated relative to image display completion
- how save/restore interacts with non-text rendering operations
- how CUD (`\x1b[NB`) behaves at the bottom margin during scroll events
- whether save/restore adjusts for scroll events (Ghostty does; others do not)
- how protocol responses are buffered and whether they contaminate subsequent queries

The solution required two orthogonal fixes:

1. **`ceil` rounding for all terminals except Warp** — images physically occupy `ceil` rows, so cursor advancement must match.
2. **`\n` scroll compensation for all terminals except Ghostty and Warp** — when CUD is clamped at the bottom margin after a scroll event, a single line feed pushes the cursor past the image.

## Recommended Next Work

The core rendering is now consistent across all five target terminals. Remaining refinements:

1. **Fix WezTerm cell size detection** — flush the terminal response buffer before querying cell size to prevent contamination from prior Kitty graphics protocol responses.
2. Consider a user override flag to force cursor row correction (`--image-row-adjust`) for edge environments.
3. Monitor terminal updates — if WezTerm or Ghostty implement `C=1` correctly in the future, it could replace the save/restore + CUD + compensation approach entirely.

---
ready: false
agent: ${env.AGENT}
---

# Feature Review: Choose One Improvements (Review 3)

## Summary

This review covers the implementation of the "Choose One Improvements" feature, which includes `FrameChrome` padding, `ChooseOne` and `ChooseMany` indicator/binding updates, horizontal layout support, hotkey badge management, and expanded CLI option sources.

While the majority of the feature is implemented correctly and with high-quality test coverage, there is a significant functional bug in the horizontal layout's hotkey badge rendering and a minor discrepancy in the CLI flag values for hotkey badges.

## Findings

### 1. Functional Bug: Horizontal Layout Hotkey Badge Collision

The implementation of hotkey badges in horizontal orientation is currently broken for multi-row layouts.

- **Issue:** `ChoiceRenderContext::render_horizontal` paints hotkey badges on the row immediately below the option (`screen_y + 1`). However, `ChoiceLayout::horizontal` packs option rows with a vertical increment of `1`.
- **Impact:** If a horizontal list wraps to multiple rows, the badges of row $N$ will overwrite the options of row $N+1$.
- **Recommendation:** `ChoiceLayout::horizontal` should be updated to account for the extra row required by badges when `hotkey_display` is not `Hidden`, or `render_horizontal` should ensure it doesn't overwrite subsequent rows. Given the spec says "placed below", the layout engine must be aware of the height requirements of the badges.

### 2. Gap: Incomplete `--hotkey-badges` Values

The specification mandates four values for the `--hotkey-badges` (alias `--hb`) flag: `hidden`, `ctrl`, `alt`, and `auto`.

- **Issue:** The current implementation in `biscuit-tui/cli/src/commands/common_choose.rs` only supports `auto`, `always` (mapping to forced `CtrlHeld`), and `never` (mapping to `Hidden`).
- **Impact:** Users cannot specifically force the display of `ALT` badges, which was a requirement in the specification.
- **Recommendation:** Update `HotkeyBadgesArg` and `resolve_hotkey_badges` to include discrete `ctrl` and `alt` modes as specified.

### 3. Minor Inconsistency: `ChooseMany` ESC Exit Code

- **Observation:** `ChooseOne` now returns exit code `0` on ESC (by restoring and submitting the initial value), which matches the "Breaking change" requirement in the spec. However, `ChooseMany` still returns exit code `1` (via `EventOutcome::Cancelled` and `ABORTED_KIND`).
- **Context:** The spec says "The default key-bindings for ChooseMany should stay as they are". Traditionally, `question` subcommands return `1` on ESC.
- **Recommendation:** Confirm if `ChooseMany` should follow `ChooseOne` in returning `0` on ESC for consistency, or if the current behavior is desired as part of "staying as they are".

### 4. Positive Observations

- **FrameChrome Padding:** Correctly implemented with defaults and per-side overrides. Well-tested.
- **Indicators:** Radio buttons for `ChooseOne` and checkboxes for `ChooseMany` are correctly implemented, including Nerd Font detection and fallback heuristics.
- **Active Selection Styling:** The `fzf`-style faint background color is correctly implemented with a 256-color palette tuned for terminal background luminance.
- **Option Sources:** The new `--csv`, `--list`, `--rows`, `--file`, and `--md` sources are implemented with robust parsing and structured record preservation (label, value, hotkey, disabled).
- **Test Coverage:** Excellent unit test coverage across the library and CLI commands.

## Conclusion

**Status: NOT READY**

The feature is not ready for production due to the **Horizontal Layout Hotkey Badge Collision** bug. This is a functional regression for multi-row horizontal layouts when hotkeys are displayed. Once the layout engine or renderer is updated to handle sub-row spacing correctly, and the missing discrete hotkey badge modes are added, the feature will be ready.

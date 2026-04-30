---
agent: "${env.AGENT}"
ready: false
---

# Review 2: Choose-One Improvements (2026-04-28)

All tests pass (453 lib + 206 CLI unit + 120 integration), but several gaps remain between the implementation and the spec/tech-design that block production readiness.

## Critical Gaps

### 1. ChooseMany Does Not Handle Explicit Hotkeys

**Spec:** Hotkeys apply to both ChooseOne and ChooseMany. Ctrl/Alt chords select and submit.

**Gap:** `ChooseManyState` lacks `ctrl_hotkeys` and `alt_hotkeys` fields, and `ChooseMany::handle_event` has no chord handling for `KeyModifiers::CONTROL` or `KeyModifiers::ALT`. Explicit hotkeys are silently ignored in ChooseMany.

**Fix:** Add the same explicit-hotkey map construction and chord handler to `ChooseManyState` and `ChooseMany::handle_event` that `ChooseOne` already has.

**Coverage:** No tests exist for ChooseMany + explicit hotkeys.

### 2. `::` Delimiter Does Not Suppress Convention Transforms

**Tech-design:** "Delimited `::` wins over convention-generated labels/values for the side it explicitly supplies. For example, with `--value snake-case`, `Red Delicious::Apple` renders `Red Delicious` and returns `Apple`, not `apple`."

**Gap:** `normalize_options` in `choice_normalize.rs` applies conventions unconditionally after the `::` split. The example above would return value `"apple"` (lower-cased by snake-case) instead of the explicit `"Apple"`.

**Fix:** Skip convention application for the side that was explicitly supplied via `::`. Only apply conventions when the side was derived (not explicitly split).

**Coverage:** No test combines `::` with conventions to verify precedence.

### 3. Outdated PTY Test for ChooseOne ESC Behavior

**Spec:** ChooseOne ESC restores the initial selection and submits (exit 0). This is a breaking change from the previous exit-1 behavior.

**Gap:** `tests/choose_cli.rs` line 745 has `esc_exits_with_code_1` which spawns `choose-one` and asserts exit code 1. The test is skipped in CI (requires `QUESTION_INTERACTIVE_PTY=1`), but if enabled it contradicts the new spec. There is no PTY test verifying that ChooseOne ESC now exits 0.

**Fix:** Update the PTY test to expect exit code 0 for ChooseOne, and add a separate PTY test for ChooseMany ESC (which should still exit 1).

## Test Coverage Gaps

### 4. Missing End-to-End Test for ChooseOne ESC → Exit 0

While `choose_one.rs` has a unit test (`run_returns_0_on_esc_with_restored_value`) that mocks the prompt closure, there is no automated test that exercises the actual event loop with an ESC key and verifies the CLI returns 0. The PTY test (see #3) is the only place this could be caught, and it asserts the old behavior.

### 5. ChooseMany Lacks Hotkey Badge Display Tests

ChooseOne has 7 tests covering hotkey badge display (modifier press/release, deadline fallback, forced mode). ChooseMany has zero equivalent tests. The badge rendering path is shared (`ChoiceRenderContext`), but the state transitions (modifier events → `hotkey_display` field) are component-specific.

### 6. No Test for Horizontal Layout + Hotkey Badge Interaction

The `choice_render.rs` test `horizontal_render_places_badge_below_row_not_inline` tests badge placement in isolation, but there are no component-level tests verifying that badges render correctly when the layout cache is populated during a real render pass in horizontal mode.

### 7. Missing Test for Numeric Hotkeys Beyond Index 10

`numeric_hotkeys_eleventh_is_alt_one` only tests index 10 (Alt+1). The spec assigns Ctrl+1-9,0 to the first 10 and Alt+1-9,0 to the next 10. There is no test verifying index 19 gets Alt+0, or that index 20+ gets no hotkey.

## Ergonomic & Polish Issues

### 8. Help Hint Text Is Misleading for ChooseOne

The default `help_hint` in `ComponentTheme` is `"Enter=Submit  Esc=Cancel"`. For ChooseOne, Esc now means "restore initial selection and submit" (exit 0), not "cancel" (exit 1). This is confusing to users.

**Fix:** Make the help hint component-aware, or at least update the ChooseOne default to something like `"Enter=Submit  Esc=Restore & Submit  Space=Select"`.

### 9. Terminal Background Detection Uses Heuristic Instead of biscuit-terminal

**Spec:** "to make the color 'faint' we must use biscuit-terminal's ability to detect the background color."

**Gap:** The implementation uses `TerminalStyle::from_env()` which reads `COLORFGBG` and `NERD_FONT` env vars. The tech-design allowed this as a fallback, but the spec explicitly called for biscuit-terminal integration.

**Fix:** Integrate `biscuit-terminal` background detection and fall back to the env heuristic only when that crate is unavailable.

### 10. Horizontal Badge Rendering May Overlap Next Option Row

In horizontal mode, badges render on `screen_y + 1` (`choice_render.rs:561`). If the terminal is tall enough to accommodate the badge row but the next option layout row also occupies that screen row, the badge will overwrite option content. The check `screen_y + 1 < area.y + area.height` only validates bounds, not content collision.

**Fix:** Either reserve an extra row per option row for badges in horizontal mode, or suppress badges when they would collide with the next layout row.

## Minor Issues

### 11. `parse_option` in `choice_normalize.rs` Is Dead Code

The standalone `parse_option` function is marked `#[allow(dead_code)]` and never called. The real pipeline uses `raw_option_to_parsed`. This function should be removed or converted into a unit-test-only helper.

## Summary & Readiness

The implementation is **not ready for production** due to:

1. **ChooseMany explicit hotkeys are completely broken** — a spec-mandated feature is missing.
2. **`::` delimiter precedence over conventions is not implemented** — violates the tech-design contract.
3. **No automated end-to-end verification of the ChooseOne ESC behavior change** — the only PTY test asserts the old (wrong) behavior.

Once #1 and #2 are fixed and the PTY test is updated, the feature will be production-ready. The test coverage gaps (#4–#7) and polish issues (#8–#10) should be addressed before a stable release but are not blockers.

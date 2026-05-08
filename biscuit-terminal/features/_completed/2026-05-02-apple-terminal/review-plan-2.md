# Implementation Plan: Review-2 Fixes for Apple Terminal Integration

## Phase 1: Single Phase — All Review-2 Findings

All four findings are in the same feature scope and share the same verification target (`cargo test -p biscuit-terminal` and `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose`). A single phase avoids intermediate states where the harness and tests disagree on color forcing.

---

### 1. High: AC-3 — Fix `used_styles` leakage in suppressed underline paths

**What:** When both double and straight underline are unsupported, the parser must emit **plain text with no escape codes at all** — including the final `\x1b[0m` reset. Currently the atomic path (`{{double-underline}}`) and block path (`<double-underline>`) set `state.used_styles = true` even when the style is suppressed due to capability checks, which causes the outer `parse_tokens` to append `\x1b[0m`.

**Files to modify:**
- `biscuit-terminal/lib/src/components/prose.rs`

**Exact changes:**

1. **Atomic suppress path** (around line 1508):  
   Remove `state.used_styles = true;` from the branch where `atomic_token_to_escape(&token).is_some()` but `atomic_token_to_escape_with_term` returned `None` (i.e., `double-underline` on a terminal with no underline support).  
   The comment already says "Emit nothing — the parser drops the styling sequence entirely", so `used_styles` must not be set.

2. **Block suppress path** (around line 1587-1604):  
   Move `state.used_styles = true;` so it executes **only when the action is NOT `BlockTagAction::Suppress`**. Currently it is set unconditionally before the `match action`. The correct logic is:
   - For `BlockTagAction::Suppress`: do NOT set `used_styles` here; let the recursive `parse_tokens_inner` call set it only if the inner content actually uses styles.
   - For `BlockTagAction::Wrap`: set `used_styles = true` as before.

   Alternatively, keep `state.used_styles = true` in the `Wrap` branch only. The `Suppress` branch already recurses into `parse_tokens_inner(&inner_content, ...)`, which will correctly set `used_styles` if the inner text contains any styled tokens.

**Tests to add/modify:**
- In `biscuit-terminal/lib/src/components/prose.rs` (unit tests):
  - `test_double_underline_suppressed_when_no_underline_support` (line ~2232): add an exact-output assertion:
    ```rust
    assert_eq!(result, "important text", "expected plain text with no escapes, got: {:?}", result);
    ```
    Also add `assert!(!result.contains("\x1b["), "must not contain any SGR escape, got: {:?}", result);`
  - `atomic_double_underline_suppressed_when_no_underline_support` (line ~2426): add the same exact-output and no-escape assertions.
- In `biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs`:
  - `no_underline_support_emits_plain_text` (line ~181): add exact-output assertion between the `---PROSE---` and `---END---` markers. Since the probe outputs:
    ```
    ---PROSE---
    important text
    ---END---
    ```
    Assert that the slice between `---PROSE---\n` and `\n---END---` equals exactly `"important text"`.

**How to verify:**
```bash
cargo test -p biscuit-terminal test_double_underline_suppressed_when_no_underline_support
cargo test -p biscuit-terminal atomic_double_underline_suppressed_when_no_underline_support
cargo test -p biscuit-terminal --test level1_apple_terminal_prose no_underline_support_emits_plain_text
```
All three must pass with exact plain-text output.

---

### 2. High: AC-2 — Wrap Level-2 double-underline output with unique sentinels

**What:** The test `level2_apple_terminal_double_underline_plain_text_visible` asserts `frame.plain.contains("important text")`, but Terminal.app's capture includes the shell transcript (the typed command line). The assertion can pass even if `bt prose` emits nothing or crashes.

**Files to modify:**
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`
- `biscuit-terminal/cli/tests/common/mod.rs` (optional, if adding a helper)

**Exact changes:**

1. Change the command sent to the harness from:
   ```rust
   send_bt_command(&mut harness, "prose '<double-underline>important text</double-underline>'");
   ```
   to a sentinel-wrapped shell pipeline:
   ```rust
   harness.send_text(b"printf '__BT_START__\\n'; bt prose '<double-underline>important text</double-underline>'; printf '\\n__BT_END__\\n'\n").expect("send_text failed");
   harness.settle();
   ```
   (Use `harness.send_text` directly rather than `send_bt_command` because the wrapper includes shell builtins.)

2. After capture, extract only the text between `__BT_START__\n` and `\n__BT_END__`:
   ```rust
   let bounded = frame.plain
       .split("__BT_START__\n")
       .nth(1)
       .and_then(|s| s.split("\n__BT_END__").next())
       .unwrap_or("");
   ```

3. Assert on `bounded` instead of `frame.plain`:
   ```rust
   assert!(
       bounded.contains("important text"),
       "expected rendered `important text` between sentinels. bounded:\n{}",
       bounded,
   );
   assert!(
       !bounded.contains("[4:2m"),
       "literal `[4:2m` fragment visible in rendered output. bounded:\n{}",
       bounded,
   );
   ```

4. Add a negative assertion that the bounded slice is non-empty, to catch the "bt crashed / emitted nothing" failure mode:
   ```rust
   assert!(!bounded.is_empty(), "sentinel-bounded output is empty — bt prose likely crashed or emitted nothing");
   ```

**How to verify:**
```bash
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose level2_apple_terminal_double_underline_plain_text_visible -- --nocapture
```
The test should pass on macOS with Terminal.app. Temporarily breaking `bt prose` (e.g., inserting a panic) should cause the empty-slice assertion to fail.

---

### 3. Medium: AC-5 — Make Drop cleanup observable in the lifecycle test

**What:** The lifecycle test verifies that the harness window disappears after Drop, but Terminal.app's "When the shell exits" preference can make this nondeterministic. The test should instead assert that `close_window` was **attempted and succeeded** (the AppleScript command returned OK), regardless of whether the physical window disappears.

**Files to modify:**
- `biscuit-test-harness/src/apple_terminal.rs`
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`

**Exact changes:**

1. **In `biscuit-test-harness/src/apple_terminal.rs`:**
   - Add a `#[cfg(test)]` or test-only field to `AppleTerminalHarness` to track cleanup outcome:
     ```rust
     pub struct AppleTerminalHarness {
         window_id: Option<i64>,
         #[cfg(test)]
         close_result: Option<std::io::Result<()>>,
     }
     ```
   - In `new()`, initialize `close_result: None`.
   - In `close_window()`, before `self.window_id.take()`, store the result:
     ```rust
     #[cfg(test)]
     let result = (|| {
         // ... existing close_window logic, but return Ok(()) on success
     })();
     #[cfg(test)]
     self.close_result = Some(result);
     ```
     (Simpler: just capture whether the `osascript` command returned `Ok(output) if output.status.success()` and store a `bool` or `Result`.)
   - Add a test-only accessor method:
     ```rust
     #[cfg(test)]
     pub fn close_succeeded(&self) -> Option<bool> {
         self.close_result.as_ref().map(|r| r.is_ok())
     }
     ```
     Alternatively, store a `close_attempted: bool` that is set to `true` when `close_window` runs, and `close_succeeded: bool` when the AppleScript exits 0.

2. **In `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`:**
   - In the lifecycle test, after the harness drops, assert:
     ```rust
     // After the inner scope drops the harness:
     assert!(
         harness.close_succeeded().unwrap_or(false),
         "harness Drop should have successfully issued close_window"
     );
     ```
   - Keep the existing window-disappearance poll as a **best-effort diagnostic** (not a hard assertion), but change the `if still_present` block to not be the sole verification. The primary verification is now `close_succeeded()`.
   - Update the comment at lines 286-295 to reflect that the observable assertion is now on `close_succeeded()`, while the physical disappearance remains best-effort due to user preferences.

**How to verify:**
```bash
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose level2_apple_terminal_harness_lifecycle -- --nocapture
```
On macOS, the test must pass with the new assertion. On non-macOS / CI, the skip path must still work.

---

### 4. Medium: AC-4 — Remove default `FORCE_COLOR` / `CLICOLOR_FORCE` from AppleTerminalHarness

**What:** `AppleTerminalHarness::spawn_shell` unconditionally injects `FORCE_COLOR=1 CLICOLOR_FORCE=1`, which causes `bt` to route through `Terminal::new_forced` and enable `osc_link_support`, collapsing the graceful-degradation path. Every Level-2 test currently needs a local `disable_color_forcing` helper to undo this. This is a footgun for future tests.

**Files to modify:**
- `biscuit-test-harness/src/apple_terminal.rs`
- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`

**Exact changes:**

1. **In `biscuit-test-harness/src/apple_terminal.rs` (line ~209):**
   Remove the hard-coded color forcing from the shell command:
   ```rust
   // BEFORE:
   shell_cmd.push_str("FORCE_COLOR=1 CLICOLOR_FORCE=1 ");
   
   // AFTER: remove this line entirely.
   ```
   Keep the `TERM` and `COLORTERM` defaults (they do not force `osc_link_support`).

2. **In `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`:**
   - Remove the `disable_color_forcing` function (lines 51-74) and its call sites in `level2_apple_terminal_link_fallback_visible` (line 96) and `level2_apple_terminal_double_underline_plain_text_visible` (line 156).
   - Remove the doc comment about `disable_color_forcing`.

3. **Update the `AppleTerminalHarness::spawn_shell` doc comment** (lines 188-194) to remove the mention of color-forcing env vars.

**How to verify:**
```bash
cargo test -p biscuit-test-harness  # must still pass
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose -- --nocapture
```
The Level-2 tests must still pass and must now exercise the actual Apple Terminal capability profile (no OSC8, no double underline) without the `disable_color_forcing` workaround.

---

## Dependencies Between Changes

| Change | Depends on |
|--------|-----------|
| 1 (AC-3 prose fix) | None — pure library change |
| 2 (AC-2 sentinel test) | 4 (harness color forcing) — tests must run with correct profile |
| 3 (AC-5 cleanup observable) | None — additive to harness API |
| 4 (AC-4 remove forced color) | None — but must be done before or with change 2 |

Because changes 2 and 4 both touch the Level-2 test file and change 4 affects the harness behavior that change 2 relies on, they are sequenced together in the single phase.

## Verification Summary

Run the following commands after all changes:

```bash
# Unit tests (must pass everywhere)
cargo test -p biscuit-terminal

# Level-1 PTY tests (must pass everywhere)
cargo test -p biscuit-terminal --test level1_apple_terminal_prose

# Harness tests (must pass everywhere)
cargo test -p biscuit-test-harness

# Level-2 real-terminal tests (macOS only, skip elsewhere)
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose -- --nocapture
```

All commands must pass. No new compiler warnings should be introduced.

---
prompt: |-
    Do a deep dive into the `cliclick` crate and:

    - describe all functionality provided by cliclick 
    - research how it can be used effectively as a testing tool in Rust projects to test "actual" terminal's resident on the host
    - research what "gotchas" or problems have developers hit when using the crate and how have they gotten around them?
    - describe all feature flags exposed by the crate and what each feature enables
    - give 4-5 examples of using cliclick for testing in a Rust project
    - find any articles or documentation if you can regarding cliclick being used with a Ratatui TUI application
        - describe any complexities that might occur when testing a Ratatui application over just a normal CLI
    - see if you can find any good information online about how target the correct terminal window so cliclick can send events to it
        - information on any terminal application you find in this regard should be captured
        - we have particular interest in Wezterm, Kitty, and iTerm2
last_updated: 2026-05-02
session_learnings: |-
    2026-05-02 — extended via real Level-3 debugging session in biscuit-tui:
    - documented set-tab-title vs set-window-title for OS NSWindow title propagation
    - added AXRaise + click pattern for cross-app keyWindow transfer
    - clarified Apple Events permission distinct from Accessibility (TCC)
    - confirmed flagsChanged limitation extends to System Events `key down`
    - added reference-implementation pointer to biscuit-tui harness
    - documented multi-window WezTerm targeting strategy
    - added `--test-threads=1` requirement and parent-app-vs-spawned-app rule
---
## cliclick Overview

> In the rusty-biscuit monorepo, `cliclick` is wrapped for Level-3 tests by the shared `biscuit-test-harness` crate (`src/cliclick.rs`). See `biscuit-test-harness/README.md` for how Level-3 keyboard injection fits with the Level-1/2 harnesses and the `RUN_LEVEL3` / `--test-threads=1` / `SpawnVisibility::Foreground` requirements.

`cliclick` is not a Rust crate on crates.io. `cargo search cliclick` and the crates.io API currently return no Rust package named `cliclick`. The project commonly meant by this name is [BlueM/cliclick](https://github.com/BlueM/cliclick), a macOS command-line tool written in Objective-C for emitting host-level mouse and keyboard events. Homebrew currently packages version `5.1`; the upstream GitHub latest release is also [`5.1`](https://github.com/BlueM/cliclick/releases/tag/5.1).

Because it is an external executable rather than a Rust crate, it has no Cargo feature flags. In Rust projects, use it by spawning the `cliclick` binary from tests or helper scripts.

## Functionality

`cliclick` sends macOS GUI input events to the currently focused application. It is therefore useful for testing behavior through the real terminal emulator instead of only through a pseudo-terminal, mock terminal backend, or captured stdout.

Supported global options from the upstream README:

- `-r`: restore initial mouse location after the command sequence finishes.
- `-m verbose`: print each action before executing it.
- `-m test`: print each action without executing it.
- `-d <target>`: target for `p` output: `stdout`, `stderr`, `clipboard`, or a file path.
- `-e <num>`: easing factor for mouse movement; higher values make mouse movement slower and more human-like.
- `-f <file>`: read commands from a file, or `-` for stdin. Lines beginning with `#` are comments.
- `-w <num>`: wait this many milliseconds after each event. The default and minimum is `20`; this is additive with explicit `w:` commands.
- `-V`: print version and release date.
- `-o`: open version history.
- `-n`: open donation flow.

Supported commands:

- `c:x,y`: left-click at coordinates.
- `rc:x,y`: right-click at coordinates.
- `dc:x,y`: double-click at coordinates.
- `tc:x,y`: triple-click at coordinates.
- `m:x,y`: move mouse.
- `dd:x,y`: start drag.
- `dm:x,y`: continue drag.
- `du:x,y`: end drag.
- `kd:keys`: key-down for modifier keys: `alt`, `cmd`, `ctrl`, `fn`, `shift`.
- `ku:keys`: key-up for those modifier keys.
- `kp:key`: press and release a named key, including arrows, return, tab, escape, delete, function keys `f1` through `f16`, page/home/end keys, numpad keys, media keys, brightness keys, volume keys, and keyboard-light keys.
- `t:text`: type text into the frontmost application.
- `w:ms`: wait.
- `p[:str]`: print text, or print current mouse position when omitted or `.`.
- `cp:x,y`: print RGB color at a screen location.

Coordinate arguments can be absolute or relative. Relative coordinates use `+` or `-`, such as `m:+50,+0`. Absolute negative coordinates, useful with displays arranged left or above the main display, can be prefixed with `=`, such as `c:100,=-200`. `.` means the current mouse position for click and drag commands.

Text typing is keyboard-layout dependent. The project documents which composed characters it can type for several layouts in [README-Characters.md](https://github.com/BlueM/cliclick/blob/master/README-Characters.md). For unsupported characters, clipboard paste or terminal-native text injection is usually more reliable.

## Using It For Rust Tests

`cliclick` is best treated as a macOS-only integration testing tool. It tests the actual resident terminal emulator and its OS integration: focus, key routing, mouse reporting, scrollback behavior, paste handling, alternate screen behavior, terminal size, font/layout edge cases, and real event timing.

A practical Rust setup usually looks like this:

- Gate tests behind `#[cfg(target_os = "macos")]`.
- Also gate them behind an env var such as `RUN_HOST_TERMINAL_TESTS=1`, because they steal focus and require local GUI permissions.
- Spawn a dedicated terminal window/tab running the test binary or example app.
- Give Terminal/iTerm2/WezTerm/Kitty Accessibility permission in macOS System Settings.
- Focus the exact target window before calling `cliclick`.
- Prefer one `cliclick` invocation with multiple commands over many short invocations; this reduces timing drift and process startup races.
- Use visible readiness markers from the app before sending input, for example a unique title, prompt text, temp file, TCP port, or log line.
- Combine input injection with a reliable observation channel: app logs, snapshot files, terminal emulator APIs, stdout capture from a child process, screenshots, or `cliclick cp:x,y` for coarse pixel assertions.

This should not replace normal Rust tests. Keep parser/state/reducer/widget tests deterministic, then add a small number of host-terminal tests for behavior that only the real terminal can prove.

## Reference implementation: rusty-biscuit `biscuit-test-harness`

The shared `biscuit-test-harness` crate contains a battle-tested macOS Level-3 harness that solves most of the problems below. Treat it as the reference implementation — copy the patterns rather than reinventing them. Live Level-3 consumers (e.g. `claudine/cli/tests/level3_*.rs`) attach to it.

Key patterns it implements:

- `TerminalHarness` trait + `WezTermHarness` that spawns into a unique window (`SpawnVisibility::Foreground` for L3), AXRaises by title, returns click coords.
- `cliclick.rs` helpers: `click_then_press`, `click_then_ctrl_chord`, `system_events_key_down/up`, `run_verbose` for diagnostic output.
- The shared `_test_l3` recipe (`just/devops.just`) sets `RUN_LEVEL3=1` so the `require_level!(Level::L3, …)` gate trips; a WezTerm parent should relaunch inside iTerm2 / Terminal.app so parent-vs-child app activation doesn't fight for OS focus.
- A "plain key delivers" diagnostic test (single arrow-down, assert the active marker moves) that localises focus issues vs chord/modifier-specific issues. Always include one of these in any Level-3 suite.

Note: prefer Level-3 cliclick **only** to verify a terminal's physical-key encoder. If you only need to prove your binary decodes a known byte sequence, inject those bytes headlessly at Level 2 (e.g. tmux `send-keys`) instead — it is reliable and never steals focus. `biscuit-tui` deliberately retired its L3 cliclick tests in favour of that approach.

## Gotchas

macOS permissions are the first failure mode. The upstream README says Terminal, iTerm, or whichever launcher invokes `cliclick` must be allowed to control the computer under Accessibility. Since `5.1`, `cliclick` warns on stderr when that permission appears to be missing.

`cliclick` targets the frontmost application, not a named process. If another window steals focus, events go there. Tests need an explicit focus step and should run in a quiet desktop/session.

Timing is brittle. Upstream issue discussions show missed or irregular `kp:` events on newer macOS versions, especially in repeated loops. The maintainer recommended batching repeated keypresses into a single `cliclick` process with `-w`, and a later timing fix was pushed to `master` after users reproduced the issue. Homebrew stable is still `5.1`, so tests that depend heavily on `kp:` may need Homebrew `--HEAD`, a pinned fork, or an AppleScript fallback.

Some modifier combinations are inconsistent. A reported issue shows `fn` working with `t:` but not reliably with `kp:` combinations such as `ctrl+fn+arrow-right`; the maintainer reproduced similar behavior with AppleScript/System Events too, suggesting this is at least partly macOS event behavior rather than only a `cliclick` bug.

**Bare-modifier presses (Ctrl alone, Alt alone) cannot be synthesised reliably from userspace on macOS.** macOS routes bare modifier state changes through AppKit's `flagsChanged` event type. cliclick's `kd:ctrl` uses `CGEventCreateKeyboardEvent`, which dispatches a regular keyDown — AppKit listeners watching `flagsChanged` (which terminals like WezTerm do for bare modifiers) don't see anything. Crucially:

- AppleScript `tell application "System Events" to key down control` shares the same underlying CGEvent path; **same limitation, not a workaround.**
- `osascript` invocations of `keystroke` likewise dispatch via the same path.
- The only known fix is a custom Rust binary built on the `core_graphics` crate that constructs a CGEvent with `CGEventType::FlagsChanged` and the relevant flag bit set, then posts via `kCGHIDEventTap`. None exists in the public Rust ecosystem as of 2026-05.

For test infrastructure: chord injection (`kd:ctrl t:r ku:ctrl`) works fine because the modifier flag rides along with the letter `keyDown`. **The bare-modifier-hold case must be marked `#[ignore]` and verified at Level 2 instead** — see [`cli-best-practices.md`](./cli-best-practices.md#known-limitation-cliclick--bare-modifier-keys-on-macos) for the canonical Level-2 workaround using raw kitty bytes via `wezterm cli send-text`.

Application behavior differs. Chrome/Chromium apps have been reported to handle drag/copy-style synthesized events differently than normal Cocoa apps. Workarounds include using app-native APIs where possible, such as AppleScript plus Chrome JavaScript for selected text, instead of pure synthetic key/mouse input.

Dragging is not universal. `dd`/`dm`/`du` can work for normal mouse interactions, but apps that distinguish mouse-move from mouse-drag events may not behave like they do with a physical pointer.

Building from source can hit macOS SDK changes. Users on macOS 15 reported `CGWindowListCreateImage` being unavailable while building; the maintainer suggested stubbing the color picker action for testing that unrelated `kp:` change.

`cp:x,y` and screenshot-like behavior may require screen capture permissions or may be affected by macOS display scaling, multiple displays, color profiles, and window shadows.

## Ratatui-Specific Notes

I did not find credible documentation or articles specifically about using `cliclick` with Ratatui applications. The closest public material is generic macOS GUI automation with `cliclick`, Ratatui’s own testing patterns, and terminal-emulator scripting docs.

Ratatui adds complexity over a normal CLI because the thing under test is not just stdout:

- Ratatui normally uses alternate screen mode, raw mode, cursor control, mouse capture, and frequent redraws.
- The terminal emulator may intercept shortcuts before the app sees them.
- Mouse behavior depends on whether the app enabled mouse capture and which mouse protocol the terminal emits.
- Layout depends on real terminal cell size, font metrics, DPI, window decorations, and terminal resize events.
- Assertions against screen pixels are more fragile than assertions against Ratatui buffers.
- A TUI may redraw asynchronously after input; tests need readiness polling, not fixed sleeps only.
- Panic cleanup matters: raw mode and alternate screen should be restored even when tests fail.

For Ratatui, keep most tests at the `ratatui::backend::TestBackend` or app-state level. Use `cliclick` only for end-to-end checks such as “real iTerm2 sends Option+Arrow as expected”, “mouse click selects the visible row”, “paste is bracketed correctly”, or “resize causes the expected layout transition”.

## Examples

### 1. Gate Host-Terminal Tests

```rust
fn host_terminal_tests_enabled() -> bool {
    cfg!(target_os = "macos")
        && std::env::var_os("RUN_HOST_TERMINAL_TESTS").is_some()
        && which::which("cliclick").is_ok()
}

#[test]
fn smoke_real_terminal_input() {
    if !host_terminal_tests_enabled() {
        eprintln!("skipping: set RUN_HOST_TERMINAL_TESTS=1 and install cliclick");
        return;
    }

    let status = std::process::Command::new("cliclick")
        .args(["-m", "verbose", "t:hello", "kp:return"])
        .status()
        .unwrap();

    assert!(status.success());
}
```

### 2. Batch Key Events Instead Of Looping Processes

```rust
#[test]
fn drive_down_arrow_repeatedly() {
    if !host_terminal_tests_enabled() {
        return;
    }

    let mut args = vec!["-w".to_string(), "75".to_string()];
    args.extend((0..10).map(|_| "kp:arrow-down".to_string()));
    args.push("kp:return".to_string());

    let status = std::process::Command::new("cliclick")
        .args(args)
        .status()
        .unwrap();

    assert!(status.success());
}
```

### 3. Test A Ratatui Selection With Real Mouse Input

```rust
#[test]
fn click_visible_row_in_real_terminal() {
    if !host_terminal_tests_enabled() {
        return;
    }

    // Assumes the test harness already opened/focused a dedicated terminal
    // and positioned the TUI at known screen coordinates.
    let status = std::process::Command::new("cliclick")
        .args([
            "-w", "100",
            "c:420,260", // row center
            "kp:return",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    // Prefer asserting through app output/log/tempfile rather than pixels.
    let selected = std::fs::read_to_string("/tmp/my-tui-selected-row").unwrap();
    assert_eq!(selected.trim(), "row-3");
}
```

### 4. Paste Text Through The Real Terminal

```rust
#[test]
fn paste_into_prompt() {
    if !host_terminal_tests_enabled() {
        return;
    }

    let text = "search query with spaces";
    std::process::Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
            child.wait()
        })
        .unwrap();

    let status = std::process::Command::new("cliclick")
        .args(["kd:cmd", "t:v", "ku:cmd", "kp:return"])
        .status()
        .unwrap();

    assert!(status.success());
}
```

### 5. Use `-m test` To Validate Generated Scripts

```rust
#[test]
fn generated_cliclick_script_is_valid_enough_to_inspect() {
    let output = std::process::Command::new("cliclick")
        .args(["-m", "test", "m:100,100", "c:.", "t:abc", "kp:return"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Move"));
    assert!(stdout.contains("Click"));
    assert!(stdout.contains("Type"));
}
```

## Targeting The Correct Terminal Window

`cliclick` itself does not target windows. It sends events to the active/focused application. The robust pattern is:

1. Create or identify a dedicated terminal window/tab/pane.
2. Set a unique title or use a terminal-specific id.
3. Use the terminal’s own automation API to focus that target.
4. Then run `cliclick`.

### WezTerm

WezTerm has a built-in CLI for running programs and manipulating panes. `wezterm cli list --format json` lists windows, tabs, panes, titles, cwd, size, and ids. `wezterm cli` chooses the target instance using `--prefer-mux`, `WEZTERM_UNIX_SOCKET`, or a running GUI instance; `--class` can help distinguish GUI instances that were started with a custom class. Pane-targeting commands typically accept `--pane-id`, defaulting to `WEZTERM_PANE` or the most recently interacted session.

Useful commands:

```bash
wezterm start --always-new-process --cwd "$PWD" -- cargo run --example my_tui
wezterm cli list --format json
wezterm cli activate-pane --pane-id "$PANE_ID"
wezterm cli set-window-title "cliclick-test-$TEST_ID" --pane-id "$PANE_ID"
```

After focusing the pane/window with WezTerm APIs, send physical-style events with `cliclick`.

Sources: [wezterm cli](https://wezterm.org/cli/cli/index.html), [wezterm cli list](https://wezterm.org/cli/cli/list.html), [wezterm start](https://wezterm.org/cli/start.html).

#### WezTerm: gotchas the docs don't tell you

Beyond the basic API, here are the things that take real debugging time to figure out:

1. **`wezterm cli set-window-title` is silently overridden by most users' `format-window-title` events** in `wezterm.lua`. The default WezTerm config plus virtually every popular config snippet derives the OS-level NSWindow title from the active *tab*'s title. Use **`wezterm cli set-tab-title --pane-id N <unique>`** instead — that propagates to what System Events / AppleScript actually sees.

2. **`tell application "WezTerm" to activate` is window-ambiguous.** With multiple WezTerm windows already open, AppleScript activates the app but picks the most-recently-keyWindow as the front window — almost never the freshly spawned one. Don't rely on this for window targeting.

3. **`wezterm cli activate-pane` only changes intra-WezTerm pane focus** — it does NOT move OS-level keyWindow between sibling WezTerm windows. Pair it with System Events `AXRaise` on a title-stamped target window.

4. **AXRaise alone often does not transfer keyWindow across applications.** When the test runner lives in a different app (e.g. iTerm2 hosting cargo), AXRaise on a WezTerm window raises it visually but doesn't make it the OS keyWindow. **A real OS click via `cliclick c:X,Y` is required** to force keyWindow assignment. Get the window's screen position from System Events (`position of targetWin`) and click into it.

5. **Parent app and spawned app should differ.** If cargo test runs in WezTerm and the tests spawn more WezTerm windows, parent and child belong to the same `NSApplication` and compete for keyWindow. Detect a WezTerm parent (`$TERM_PROGRAM == "WezTerm"`) and relaunch cargo inside iTerm2 / Terminal.app via `osascript`.

6. **`AXFocusedWindow` ≠ `AXMain`.** Probe `AXFocusedWindow` — that's the actual keyWindow where keyboard events go. `AXMain` is for app-level menu actions and can point at a different window than the one receiving keyboard events. Your diagnostic queries should target `AXFocusedWindow`.

7. **macOS multi-monitor coords aren't bugs.** A 2× 5K display arrangement is 5120 px wide; window x-positions in the 4000s are valid. Don't treat large coordinates as off-screen failures without checking the user's display configuration.

8. **`config.enable_kitty_keyboard = true` is required in the user's `wezterm.lua`** for WezTerm to honor the binary's `PushKeyboardEnhancementFlags`. Without it, bare-modifier reporting is silently disabled — even when everything else (focus, click, AXRaise) is correct.

### Kitty

Kitty supports remote control with `kitten @`. `kitten @ ls` returns JSON describing OS windows, tabs, and kitty windows, including ids, titles, cwd, process id, and command line. `kitten @ focus-window --match 'title:^Output'` can focus a matching kitty window, and `--match` supports fields such as `id`, `title`, `pid`, `cwd`, `cmdline`, `env`, `var`, `state`, `session`, and `recent`.

Useful commands:

```bash
kitty --title "cliclick-test-$TEST_ID" cargo run --example my_tui
kitten @ ls
kitten @ focus-window --match "title:^cliclick-test-$TEST_ID$"
```

Remote control must be enabled or scoped correctly. Kitty documents `allow_remote_control` as broad and recommends fine-grained permissions with `remote_control_password` when possible.

Sources: [Kitty remote control](https://sw.kovidgoyal.net/kitty/remote-control/), especially the sections on matching windows and `focus-window`.

### iTerm2

iTerm2 has AppleScript support and a Python API. The Python API exposes `Window.async_activate()`, which gives a window keyboard focus and orders it to the front, and notes that you may also need to activate the app itself. `Session.async_activate(select_tab = true, order_window_front = true)` can make a session active in its tab and bring the window forward.

Useful approaches:

```applescript
tell application "iTerm2"
    activate
    -- Select a known window/session here, then bring it forward.
end tell
```

For more precise targeting, prefer the Python API: enumerate app windows/tabs/sessions, find the session/window by id, title, profile, cwd, or a variable, call `app.async_activate()`, then `window.async_activate()` or `session.async_activate()`.

Sources: [iTerm2 scripting](https://iterm2.com/documentation-scripting.html), [iTerm2 Python Window API](https://iterm2.com/python-api/window.html), [iTerm2 Python Session API](https://iterm2.com/python-api/session.html), [iTerm2 Focus API](https://iterm2.com/python-api/focus.html?highlight=monitor).

### macOS Terminal.app And Generic Fallback

For Terminal.app or generic macOS apps, use AppleScript/System Events to activate the app and select a window where possible:

```bash
osascript -e 'tell application "Terminal" to activate'
```

This only focuses the app, not necessarily a specific tab or process. For reliable tests, prefer a terminal with addressable windows/panes, or launch a fresh dedicated window and keep the desktop quiet.

## Recommended Testing Strategy

Use `cliclick` for a thin layer of host-terminal integration tests, not as the main test framework. Keep the high-volume checks in pure Rust: app reducers, input mapping, parser logic, Ratatui `TestBackend` rendering, and snapshot tests. Then add a small macOS-only suite that proves the real terminal emulator sends and receives the events you care about.

The most reliable host-terminal tests are those that avoid pixel-perfect assertions. Drive the actual terminal with `cliclick`, but assert through structured side channels: log files, temp files, stdout from the child process, terminal remote-control `get-text` APIs, or app-specific debug hooks.

## Sources

- [BlueM/cliclick README](https://github.com/BlueM/cliclick)
- [cliclick latest release 5.1](https://github.com/BlueM/cliclick/releases/tag/5.1)
- [Homebrew cliclick formula](https://formulae.brew.sh/formula/cliclick)
- [cliclick character support](https://github.com/BlueM/cliclick/blob/master/README-Characters.md)
- [cliclick issue 164: `kp` timing problems](https://github.com/BlueM/cliclick/issues/164)
- [cliclick issue 177: `kp:` problems on Sonoma](https://github.com/BlueM/cliclick/issues/177)
- [cliclick issue 188: `fn` modifier with `kp:`](https://github.com/BlueM/cliclick/issues/188)
- [cliclick issue 190: copy simulation brittleness](https://github.com/BlueM/cliclick/issues/190)
- [WezTerm CLI targeting docs](https://wezterm.org/cli/cli/index.html)
- [Kitty remote control docs](https://sw.kovidgoyal.net/kitty/remote-control/)
- [iTerm2 scripting docs](https://iterm2.com/documentation-scripting.html)
- [iTerm2 Python API docs](https://iterm2.com/python-api/)

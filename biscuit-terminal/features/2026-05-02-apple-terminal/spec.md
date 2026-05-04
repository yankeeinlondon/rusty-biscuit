# Apple Terminal Integration Tests & Prose Graceful Degradation

Add integration tests targeting Apple Terminal — a lower-capability terminal — and implement the corresponding graceful-degradation logic in `Prose`.

## Terminal Capability Profile

```sh
Basic Info
  App:        AppleTerminal
  OS:         macOS
  Size:       153 x 62
  Is TTY:     yes
  In CI:      no

Repository
  In Repo:    yes
  Monorepo:   no
  Root:       /Users/ken/.

Fonts
  Name:       n/a
  Size:       n/a
  Nerd Font:  unknown
  Ligatures:  unlikely

Colors
  Depth:      TrueColor
  Mode:       Light
  Background: #ffffff (255, 255, 255)
  Foreground: #000000 (0, 0, 0)
  Cursor:     #000000 (0, 0, 0)

Features
  Italics:      yes
  Images:       None
  OSC8 Links:   no
  OSC10 FG:     no
  OSC11 BG:     no
  OSC12 Cursor: no
  OSC52 Clip:   no
  Mode 2027:    no

Underline Support
  Straight:   yes
  Double:     no
  Curly:      no
  Dotted:     no
  Dashed:     no
  Colored:    no

Multiplexing
  Type:       None

Connection
  Type:       Local

Locale
  Raw:        en_US.UTF-8
  Tag:        en-US
  Encoding:   Utf8

Config
  File:       /Users/ken/Library/Preferences/com.apple.Terminal.plist
```

## What Is Being Implemented

Graceful-degradation logic inside `Prose` so that unsupported escape sequences are never emitted against Apple Terminal (or any terminal that reports the same capability profile).

### Graceful Degradation Behaviors

All behaviors are implemented via **inline capability checks in `Prose` tag handlers**, following the existing `<a>` tag pattern in `prose.rs`.

| Feature | Capability Check | When Unsupported | Fallback Output |
|---------|------------------|------------------|-----------------|
| OSC8 hyperlinks | `osc_link_support == false` | Emit markdown-style link | `[{description}]({reference})` |
| Double underline | `underline_support.double == false` AND `underline_support.straight == true` | Emit single underline | `\x1b[4m` |
| Double underline | `underline_support.double == false` AND `underline_support.straight == false` | Emit plain text | no escape codes |

> **Out of scope:** Refactoring `Prose` to use the `Style`/`Stylist` system. Add a TODO comment in `prose.rs` noting that `Prose` and `Style` should eventually converge.

## What Is Being Tested

### Test Strategy — Two Tiers

#### Tier 1 — Level-1 PTY Tests

Strict byte-level assertions against a spoofed Apple Terminal environment.

- **Mechanism:** Spawn a PTY with `TERM_PROGRAM=Apple_Terminal` set in the environment (and optionally other capability env vars).
- **Pattern:** Follow existing `level1_mode_2027.rs` tests.
- **Assertions:**
  - No OSC8 escape sequences (`\x1b]8;;`) are present in the output.
  - No double-underline SGR (`\x1b[4:2m`) is present in the output.
  - Markdown-style links (`[desc](url)`) appear when OSC8 is unsupported.
  - Single-underline SGR (`\x1b[4m`) appears when double underline is requested but only straight underline is supported.

#### Tier 2 — AppleScript Harness (Real Display Path)

End-to-end validation through the actual Terminal.app process.

- **Spawn:** `osascript -e 'tell application "Terminal" to do script "..."'` (returns tab ID).
- **Background:** Spawn the window without grabbing focus by snapshotting the frontmost application before `do script` and re-activating it immediately afterwards. The Terminal.app window remains in normal window-manager z-order but sits behind the developer's foreground app — no Dock minimize animation, no risk of stray keystrokes landing in the test window.
- **Capture:** `osascript -e 'tell application "Terminal" to get contents of selected tab of window id N'`.
  - Terminal.app returns **plain text only**.
  - For this backend `frame.raw == frame.plain`; escape sequences are not observable.
- **Assertions (content-match):**
  - Verify captured text **contains** expected fallback strings (e.g., `[click here](https://example.com)` appears in output when OSC8 is unsupported).
  - Verify captured text **does NOT contain** unexpected escape sequences rendered as literal characters (e.g., no `\x1b]8;;` or `\x1b[4:2m` visible in output).
  - Verify harness lifecycle works: spawn succeeds, capture returns non-empty content, and cleanup runs on `Drop` without manual intervention.
- **Cleanup:** Close the tab on `Drop` of the harness.
- **Skip conditions:**
  - Terminal.app is unavailable (not installed or not on macOS).
  - `CI=1` is set in the environment.

## Test Fixtures

Concrete input/output pairs for the two graceful-degradation behaviors.

### 1. OSC8 Link Fallback

| | Value |
|---|---|
| **Input** | `<a href="https://example.com">click here</a>` |
| **Expected Tier 1 output** | `[click here](https://example.com)` |
| **Expected Tier 2 capture** | Contains `click here` and `(https://example.com)` (exact format may vary by terminal rendering) |
| **Must NOT contain** | `\x1b]8;;https://example.com\x1b\\` or `\x1b]8;;https://example.com\x07` |

### 2. Double Underline Fallback

| | Value |
|---|---|
| **Input** | `<double-underline>important text</double-underline>` |
| **Expected Tier 1 output (straight supported)** | `\x1b[4mimportant text\x1b[0m` |
| **Expected Tier 1 output (straight NOT supported)** | `important text` (no escape codes) |
| **Must NOT contain** | `\x1b[4:2m` |
| **Expected Tier 2 capture** | Contains `important text`, does not contain visible escape sequence garbage |

## Acceptance Criteria

- [ ] **AC-1:** When `osc_link_support=false`, `Prose` emits `[description](url)` instead of an OSC8 sequence or raw URL.
- [ ] **AC-2:** When `underline_support.double=false` and `underline_support.straight=true`, `Prose` emits `\x1b[4m` for a double-underline request.
- [ ] **AC-3:** When `underline_support.double=false` and `underline_support.straight=false`, `Prose` emits no underline escape codes.
- [ ] **AC-4:** Level-1 PTY tests with `TERM_PROGRAM=Apple_Terminal` pass and assert the exact byte sequences above.
- [ ] **AC-5:** Level-2 AppleScript harness spawns Terminal.app, captures output, and cleans up without manual intervention.
- [ ] **AC-6:** Level-2 tests are skipped automatically when Terminal.app is unavailable or `CI=1`.

## Risks & Limitations

1. **Plain-text-only capture (Tier 2):** Because Terminal.app returns only visible text, we cannot directly verify that *no* OSC8 bytes were emitted — only that the visible output contains the expected text without garbage. Tier 1 covers the negative byte-level assertion.
2. **macOS-only:** Apple Terminal is only available on macOS. Tier 2 tests will always skip on Linux and Windows.
3. **Window visibility:** Even with minimization, Terminal.app may briefly appear. The harness minimizes as soon as possible after spawn.
4. **No `CI` coverage:** Tier 2 is skipped in CI; only Tier 1 runs in CI.

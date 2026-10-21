# Graphics Where Possible: Human Verification

These checks need a person at a real terminal. The plan did not run them;
they are not completion criteria, and a failure is fixed forward. Record each
result on its blank lines: the terminal and its version, the multiplexer and
its version (or "none"), and pass or fail with what you saw.

## Before you start

- Build `bt` from this branch on every host you test from (the local Mac,
  `build-linux`, and `build-win-native`):

  ```sh
  cargo install --path biscuit-terminal/cli --locked
  ```

- Every check below starts by running `bt` with no arguments in the terminal
  under test. Read its `Images:` line: it names the route's format,
  transport, placement, confidence, and the reason. Write that line into the
  result; it says what `bt` decided, which separates a wrong decision from a
  rendering problem.
- `bt image --example` writes a 16×16 test PNG to the temp directory and
  draws it 16 columns wide. Any other image works too: `bt image <file>`.
- `bt flowchart --example` and `bt bar-chart --example` draw a diagram.
- Run each command directly in the terminal, never piped: piped output is
  always plain text by design.
- When there are no graphics and a setting could fix it, `bt` prints a
  one-line hint to stderr naming the fix. Note any hint in the result.

## 1. iTerm2

**Setup:** local iTerm2, no multiplexer.

```sh
bt
bt image --example
bt flowchart --example
```

**Look for:**

- `Images:` reports format `Iterm2`, transport `Direct`.
- The image and the diagram render as pictures.
- No `Gi=…;OK` text (or any other escape garbage) appears at the next shell
  prompt after each command.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 2. Windows SSH server

**Setup:** WezTerm on macOS, connected with `ssh build-win-native`, running
the Windows build of `bt` inside that SSH session (the system console host,
not WezTerm's multiplexer).

```powershell
bt
bt bar-chart --example
```

**Look for:**

- `Images:` reports format `Iterm2` (`Known`); the reason names the Windows
  SSH server and WezTerm.
- The chart renders as a picture.
- No blank rows where the chart should be, and no blank rows below it.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 3. Windows Terminal

**Setup:** native Windows Terminal (1.22 or later) on `build-win-native`, a
local PowerShell tab. Run the chart once with a dark color scheme and once
with a light one.

```powershell
bt
bt bar-chart --example
```

**Look for:**

- `Images:` reports format `Sixel`, transport `Direct`; the reason cites the
  DA1 reply's attribute `4` or the Windows Terminal version.
- The chart renders as a Sixel picture.
- The chart's theme matches the background: a dark chart on the dark scheme,
  a light chart on the light scheme, with no mismatched band around it.
- Note the Windows Terminal version (`wt --version` or Settings > About).

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line (dark scheme): ______________________
- `Images:` line (light scheme): ______________________
- Pass / fail, notes: ______________________

## 4. mintty

**Setup:** Git Bash (mintty) on `build-win-native`, no multiplexer.

```sh
bt
bt image --example
```

**Look for:**

- `Images:` reports format `Iterm2`; the reason names mintty.
- The image renders as a picture.
- Also, if you can: start `bash.exe` from Git for Windows in a plain console
  window (not mintty, not Windows Terminal) and run `bt image --example`.
  `bt` reads that as mintty and sends the iTerm2 format, which the console
  is expected to drop silently. Record whether any escape garbage appears.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 5. Ghostty under tmux

**Setup:** tmux 3.3 or later inside Ghostty, with passthrough on:

```sh
tmux set -g allow-passthrough on
```

Then, inside a tmux pane:

```sh
bt
bt image --example
```

Afterwards resize the pane (split it, or resize the Ghostty window), switch
to another tmux window and back, and scroll in copy mode.

**Look for:**

- `Images:` reports format `Kitty`, transport `Tmux`, placement
  `Placeholders`.
  - If placement is `Ordinary`, the reason says tmux is not sending UTF-8 or
    24-bit color to a client; add `set -as terminal-features
    ',xterm-ghostty:RGB'` and rerun.
- The image follows the pane: it moves with its rows on resize and scroll,
  disappears when you switch windows, and returns when you switch back.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 6. Kitty under tmux

**Setup:** the same as check 5, in Kitty instead of Ghostty
(`set -g allow-passthrough on`; if placement is `Ordinary`, add
`set -as terminal-features ',xterm-kitty:RGB'`).

```sh
bt
bt image --example
```

Then resize the pane, switch tmux windows and back, and scroll in copy mode.

**Look for:**

- `Images:` reports format `Kitty`, transport `Tmux`, placement
  `Placeholders`.
- The image follows the pane, as in check 5.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 7. WezTerm under tmux

**Setup:** tmux 3.3 or later inside WezTerm, with
`tmux set -g allow-passthrough on`.

```sh
bt
bt image --example
```

Then resize the pane and switch tmux windows and back.

**Look for:**

- `Images:` reports format `Kitty`, transport `Tmux`, placement `Ordinary`.
- The image renders.
- What you see after resizing and switching matches the documented limits of
  ordinary placement through tmux (`docs/discovery/graphics-routes.md`,
  "Known limits of ordinary placement through tmux"): the image may drift on
  resize or copy-mode scroll and may stay visible over another window after a
  switch. Record anything worse than that.
- With `tmux set -g allow-passthrough off`, rerun `bt image --example`: no
  image, and a hint naming `set -g allow-passthrough on`.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 8. SSH from inside a local tmux

**Setup:** tmux inside WezTerm on the Mac, with
`tmux set -g allow-passthrough on`. In a tmux pane, `ssh build-linux` (no tmux
on the far side), then on `build-linux`:

```sh
echo "$TERM"   # expect tmux-256color or screen-256color
bt
bt image --example
```

If no image appears, rerun with the override the hint names:

```sh
BISCUIT_GRAPHICS_TRANSPORT=tmux bt image --example
```

**Look for:**

- Either the image renders after the wrapped probe (`Images:` reports
  transport `Tmux`, confidence `Verified`), or there is no image and the
  stderr hint names `BISCUIT_GRAPHICS_TRANSPORT=tmux`.
- With the override set, the image renders.
- Known, not a failure: the pane title may change to text starting `Gi=`,
  because the unwrapped half of the probe reaches the local tmux, which reads
  it as a title change. Record whether it happens.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `TERM` on `build-linux`: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 9. Zellij 0.45+

**Setup:** Zellij 0.45 or later inside WezTerm or Ghostty
(`zellij --version`).

```sh
bt
bt image --example
```

**Look for:**

- `Images:` reports format `Kitty`, placement `Ordinary`, confidence
  `Verified`.
- The image renders.
- If there is no image, record the reason: an older Zellij names the upgrade,
  and a reason about the cell size means `bt` could not measure the cells.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line: ______________________
- Pass / fail, notes: ______________________

## 10. Terminal.app

**Setup:** macOS Terminal.app, no multiplexer.

```sh
bt
bt image --example
bt flowchart --example
```

**Look for:**

- `Images:` reports format `None`. Also record the color depth `bt` reports
  for Terminal.app.
- When the color depth is truecolor, the image draws as rows of colored `▀`
  half-block cells; otherwise it prints `[Image: bt-example-tiny.png]`.
- The diagram shows its source as a code block, never half-blocks.
- No escape garbage, either in the output or at the next shell prompt.

**Result:**

- Terminal + version: ______________________
- Multiplexer + version: ______________________
- `Images:` line and color depth: ______________________
- Pass / fail, notes: ______________________

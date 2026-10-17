# Level 3: OS Key Injection

Part of the `biscuit-test-harness` skill; [SKILL.md](SKILL.md) is the index.

Level 3 is the only way to prove what bytes a terminal emits when the user
presses a chord. Each platform has one injector, and each `available()` skips
cleanly off its platform.

| Module | Platform | Mechanism |
|---|---|---|
| `src/cliclick.rs` | macOS | `cliclick` (Quartz events), System Events for modified chords |
| `src/xdotool.rs` | Linux X11 | `xdotool key` through XTEST |
| `src/win_input.rs` | Windows | PowerShell `SendKeys.SendWait` |
| `src/xvfb.rs` | Linux | XTEST on a private `Xvfb` display with its own Kitty |

## Rules

- **Select the window by a unique title match.** Zero or several matches is an
  error, never a first-match guess: a key sent to the wrong window looks like a
  broken product.
- **The window must be focused**: `SpawnVisibility::Foreground` plus
  `focus_spawned_pane()`. Poll until it is genuinely frontmost; a raise returns
  before the window becomes key.
- **macOS focus is global**: gate on `RUN_LEVEL3=1` and run one test at a time.
- **Send a modified chord as one event.** AppleScript `keystroke … using <mod>
  down` carries the modifier on the key event; `cliclick`'s separate key-down,
  key, and key-up events race. Bare-modifier presses are unreliable through
  `cliclick`; prove them at L2 with raw Kitty keyboard bytes.
- **Never pass `--window` to `xdotool`.** It switches to `XSendEvent`, whose
  events terminals ignore as synthetic. Focus the window, then inject globally.
- **When an L3 test is unreliable, fix the harness**, not the test.

## A key press without taking the user's focus

- **Linux:** `xvfb::XvfbKitty` starts its own `Xvfb` and Kitty, focuses Kitty
  on that display, and presses through XTEST (`press_ctrl('c')`,
  `press_escape()`). It is still L3, because the press is an OS key event.
  Example: `claudine/cli/tests/level3/level3_drain_ctrl_c.rs`.
- **macOS and Windows** have no private desktop: OS key events reach only the
  focused session. The closest no-focus press is `KittyHarness::send_key` on a
  `KittyInstance`, which Kitty encodes as it would a physical press. That skips
  the OS input layer, so it is L2 evidence in a `level2_` file. Example:
  `claudine/cli/tests/level2/level2_drain_ctrl_c_kitty.rs`.

## Never steal focus outside a `level3_` file

`Foreground` and `focus_spawned_pane()` raise a terminal over whatever the user
is doing. Only the `level3_` file prefix keeps them behind the L3 filterset and
`RUN_LEVEL3=1`; in any other file they run unannounced during `just test` or
`just test-l2`.

- Claudine enforces this at L1 in
  `claudine/cli/tests/l1/test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files`;
  copy that guard into any area that grows L3 coverage.
- A test *name* must not contain `level3_` unless it is L3: the filtersets
  match the substring, so the test silently drops out of every other recipe.
- `just test-l3` refuses to start without a TTY unless `BISCUIT_L3_TAKE_FOCUS=1`,
  and prompts first from a terminal. Never set that variable on a human's
  behalf; it exists for someone who knows the machine is free.

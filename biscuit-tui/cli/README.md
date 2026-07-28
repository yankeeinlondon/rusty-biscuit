# question

A CLI for the `biscuit-tui` library. Exposes each input component as a subcommand, making them directly usable in shell scripts and pipelines.

## Installation

```bash
cargo install --path .
```

The binary is named `question`.

## Global Flags

- `--output {raw|json|null}` — controls output serialization (default: `raw`)
  - `raw`: plain text, one value per line (default for scalars and multi-selects)
  - `json`: JSON encoding (strings are quoted, arrays for multi-value outputs)
  - `null`: NUL-byte separated (useful when values may contain newlines; pairs with `xargs -0`)
- `--height <N|PCT%>` — render inline in up to the given number of rows below the cursor instead of fullscreen. Accepts either an absolute cell count (e.g. `12`) or a percentage (e.g. `50%`). Both forms are treated as a maximum: when the live terminal is smaller, the prompt clamps to the rows actually available. Percentages are resolved against the current terminal rows (floor of 3) and **re-resolved on every terminal resize** so the inline viewport tracks the requested fraction mid-prompt.
- `--show-input-on-exit` — preserve the rendered prompt on exit instead of clearing it. Default behaviour is fzf-style: the inline viewport is wiped on submit/Esc/Ctrl-C so the terminal reclaims the space. With this flag set the final frame stays on screen and the cursor moves to the row just below the chrome, so subsequent shell output follows the rendered border without overlapping it. No effect on fullscreen prompts.

## Subcommands

### text-input

Single-line text input.

```bash
question text-input --label "Enter your name" --max-length 50
```

**Options:**

- `--label <TEXT>` — label text
- `--label-position {above|below|left|right}` — where the label renders (default: `above`)
- `--max-length <N>` — maximum input length (hard cap enforced at keystroke time)

**Output:** raw string followed by newline.

### text-area-input

Multi-line text editor.

```bash
question text-area-input --width 80 --scrollbar
```

**Options:**

- `--label <TEXT>` — label text
- `--width <N>` — editor width in columns (default: 60)
- `--scrollbar` — show vertical scrollbar when content exceeds height
- `--initial <TEXT>` — initial buffer contents (newlines in the argument become line breaks)

**Output:** raw text (newlines preserved).

### boolean-switch

Toggle switch.

```bash
question boolean-switch --labels "YES,NO" --initial true
```

**Options:**

- `--label <TEXT>` — label text
- `--labels <ON,OFF>` — custom on/off captions (default: `"true,false"`)
- `--initial {true|false}` — initial checked value (default: `false`)

**Output:** `true` or `false`.

### choose-one

Single-selection list.

```bash
question choose-one Red Green Blue
printf "%s\n" "Red" "Green" "Blue" | question choose-one
```

**Option sources (mutually exclusive):**

- Positional arguments — used when no explicit source flag is set
- `--csv <TEXT>` — comma-separated list (alias: `--options` for backward compatibility)
- `--list <TEXT>` — newline-separated list
- `--rows <TEXT>` — newline-separated `label::value` pairs
- `--file <PATH>` — JSON, JSONL, NDJSON, YAML, TOML, or CSV file containing an array
- `--md <PATH> <PROP>` — YAML frontmatter array property from a Markdown file
- Piped stdin (automatic when stdin is not a TTY)

TOML files must use a top-level `options` array. Entries may be strings,
inline tables (`options = [{ label = "Red", value = "apple" }]`), or
array-of-tables records (`[[options]]`) with `label`, `value`, `hotkey`, and
`disabled` fields.

**Selection & filtering:**

- `--delimiter <CHAR>` — split each option on the first delimiter into `label` and returned `value`
- `--label <TEXT>` — label text
- `--label-position {above|below|left|right}` — where the label renders (default: `above`)
- `--selected <VALUE>` — pre-select the option whose value matches
- `--required` — submission is blocked if no selection made
- `--no-filter` — disable the default fuzzy filter (alphanumeric keys are then ignored)
- `--sort {natural|inverse|asc|desc}` — order options before rendering (`reverse` is a hidden alias for `inverse`)

**Hotkeys & normalization:**

- `--numeric-hot-keys` — auto-assign Ctrl+1..9,0 then Alt+1..9,0 to the first 20 options
- `--label-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>` — transform option labels
- `--value-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>` — transform option values
- `::` delimiter in option text splits `label::value` (takes precedence over conventions)
- `[CTRL+X]`, `[ALT+X]`, `[OPT+X]` prefixes in option text assign explicit hotkeys

**Showing hotkey badges:**

- Holding a bare `Ctrl` or `Alt` key reveals all matching badges with the held modifier emphasised — *requires a terminal that emits kitty-protocol bare-modifier events*. WezTerm needs `enable_kitty_keyboard = true` in `wezterm.lua` AND a full restart for the config change to load. kitty.app supports it out of the box.
- Portable fallback: `Ctrl+Space` and `Alt+Space` toggle the corresponding emphasis without needing kitty-protocol modifier events.
- **macOS gotcha**: by default macOS binds `Ctrl+Space` to "Select previous input source" — the chord is eaten by the OS before it reaches the terminal. Disable it in *System Settings → Keyboard → Keyboard Shortcuts → Input Sources*.
- For diagnosis, run with `BISCUIT_TUI_TRACE_KEYS=1` and tail `$TMPDIR/biscuit-tui-keys.log`. Every key event the binary actually receives is logged. If holding bare Ctrl produces no log entries, your terminal isn't emitting kitty bare-modifier events.

**Chrome:**

- `--border`, `--border-label <TEXT>`, `--border-style <STYLE>` — add border chrome
- `--margin <N>`, `--mt <N>`, `--mb <N>`, `--ml <N>`, `--mr <N>` — outer margin
- `--padding <N>` / `-p <N>`, `--pt <N>`, `--pb <N>`, `--pl <N>`, `--pr <N>` — inner padding
- `--active-color {grey|green|yellow|red}` — background colour for the active row (default `grey`); the renderer picks a contrasting foreground based on the detected terminal background and only paints the focus indicator + label + one trailing blank cell

**Output:** the selected value (raw string).

When no explicit source flag is provided, `choose-one` reads options
from positional arguments first, then from piped stdin. Without
`--delimiter`, each option's label and value are the same. With
`--delimiter ":"`, `question choose-one "Apple:1"` displays `Apple`
and returns `1`.

Typing alphanumeric characters opens the fuzzy filter by default. Use
Up/Down (or j/k) to move the active row, Space to select, and Enter to
submit. `Esc` restores the initial selection and submits (exit `0`).
Ctrl/Alt hotkeys select and submit immediately.

`--height` accepts a percentage suffix — see **Global Flags**.

### choose-many

Multi-selection list.

```bash
question choose-many Red Green Blue --min-selections 1 --max-selections 2
printf "%s\n" "Red" "Green" "Blue" | question choose-many
```

**Option sources:** Same set as `choose-one` (positional, `--csv`, `--list`, `--rows`, `--file`, `--md`, stdin).

**Selection & filtering:**

- `--selected <VALUE>` — pre-select values; repeat the flag to pre-select multiple (`--selected foo --selected bar`). Comma-splitting is **not** applied — if you need CSV semantics, use the deprecated `--initial` flag.
- `--required` — fail if no items are selected.
- `--min-selections <N>` — minimum number of selections required (submit-time validation)
- `--max-selections <N>` — maximum number of selections allowed (keystroke-time cap)
- `--delimiter <CHAR>` — split each option on the first delimiter into `label` and `value`
- `--no-filter` — disable fuzzy filter
- `--sort {natural|inverse|asc|desc}` — order options before rendering

**Hotkeys, normalization, and chrome:** Same flags as `choose-one`.

**Output (raw mode):** one value per line (newline-separated, matches `grep` and `sort` conventions).

**Output (json mode):** JSON array of selected values.

**Output (null mode):** NUL-separated list (for `xargs -0`).

Use Up/Down to move the active row, Space to toggle it, Enter to
submit the current selection exactly as-is, `Ctrl+A` to select all
enabled options, and `Ctrl+D` to clear the selection.

`--height` accepts a percentage suffix — see **Global Flags**.

### completions

Generate shell completion scripts.

**Install steps:**

```bash
# zsh
question completions zsh > "${fpath[1]}/_question"
# Restart shell or: autoload -U compinit && compinit

# bash
question completions bash > /usr/local/etc/bash_completion.d/question
```

> **Note:** The `question` binary must be installed *before* the completion script is sourced. clap_complete calls `_question` against the binary's `--help` to generate dynamic completions.

**Supported shells:**

- `bash`, `zsh`, `fish`, `elvish`, `powershell`

**Hotkey-prefix completion (zsh/bash only):**
When typing positional options for `choose-one` or `choose-many`, entering `[` followed by `<TAB>` offers `[CTRL+`, `[ALT+`, and `[OPT+` as completion candidates. This is supported in zsh and bash via a dedicated positional completer; other shells fall back to standard clap_complete output.

### input-table

Grid of mixed input cells.

```bash
question input-table --columns '[
  {"type": "static", "id": "name", "text": "Alice"},
  {"type": "boolean", "id": "active"}
]'
```

**Options:**

- `--columns <JSON>` — array of column definitions. Each object has:
  - `type` — one of: `static`, `boolean`, `text`, `textarea`, `choose-one`, `choose-many`
  - `id` — column identifier (becomes JSON key in output)
  - type-specific fields (e.g. `text` for static, `options` for choice columns, `max_length` for text)
- `--rows <JSON>` — optional initial row data (array of arrays matching column order)

**Column spec schema:**

```json
{
  "type": "static",
  "id": "name",
  "text": "Display text"
}

{
  "type": "boolean",
  "id": "active",
  "initial": true
}

{
  "type": "text",
  "id": "email",
  "max_length": 100
}

{
  "type": "textarea",
  "id": "notes",
  "width": 40
}

{
  "type": "choose-one",
  "id": "role",
  "options": ["Admin", "User", "Guest"],
  "required": true
}

{
  "type": "choose-many",
  "id": "tags",
  "options": ["urgent", "bug", "feature"],
  "min_selections": 1,
  "max_selections": 3
}
```

**Output (raw mode):** JSON array of row objects. Each object is keyed by `column_id`:

```json
[
  {
    "name": "Alice",
    "active": true,
    "role": "Admin",
    "tags": ["urgent", "bug"]
  },
  {
    "name": "Bob",
    "active": false,
    "role": "User",
    "tags": []
  }
]
```

Note that boolean cells emit JSON booleans (`true`/`false`), and `choose-many` cells emit JSON arrays. This typed output was introduced in Phase 5.

**Output (null mode):** `key=value` pairs separated by NUL bytes. Multi-selection cells emit one `key=value` pair per selected value.

## Exit Codes

- `0` — user submitted a value (stdout contains the result). For `choose-one`, pressing `Esc` restores the initial selection and also exits `0`.
- `1` — user pressed Esc to abort (all components except `choose-one`), or a terminal I/O error occurred. No output written to stdout on abort.
- `130` — user pressed Ctrl-C / SIGINT (no output written to stdout)
- Non-zero (other) — argument parsing error or invalid configuration

## Shell Integration

### Capture scalar values

```bash
NAME=$(question text-input --label "Name")
ACTIVE=$(question boolean-switch --labels "Yes,No")
ROLE=$(question choose-one Admin User Guest)
```

### Process multi-selection list

```bash
question choose-many Red Green Blue | while read -r color; do
  echo "Selected: $color"
done
```

### Handle NUL-separated values (when values may contain newlines)

```bash
question choose-many --output null --options "Line one,Line\ntwo" | xargs -0 -I {} echo "Item: {}"
```

### Parse JSON table output

```bash
question input-table --columns '[{"type":"text","id":"name"},{"type":"boolean","id":"active"}]' \
  | jq '.[] | select(.active == true) | .name'
```

## Verification Gates

Run `just test-pty` from `biscuit-tui/` to execute the env-gated PTY/shell
verification suites (keyboard protocol, completions shell, choose-cli PTY)
required by the Verification Gates contract.

### Test Rigor — Level 1 / Level 2 / Level 3

Test count is not test rigor. A feature with hundreds of unit tests can still
ship with a glaring user-visible bug if none of the tests exercise the right
layer. Every user-observable requirement must be classified against these three
levels:

| Level | Mechanism | What it proves |
|-------|-----------|----------------|
| **1** | Unit tests + PTY (`expectrl`) with manufactured input bytes | Internal state transitions, byte-level parsing, rendering math. Cannot prove the terminal's encoder fires correctly — *you* generate the bytes. |
| **2** | Spawn binary in real terminal (`wezterm cli` / `kitty @` / `tmux`); capture rendered pane text via the terminal's own CLI | Glyphs, widths, SGR styling, scroll, cursor position render correctly through a real terminal. Input is still byte-injected, so the terminal's input encoder isn't exercised. |
| **3** | Real OS keyboard injection (`cliclick` on macOS, `xdotool` on Linux) into the spawned terminal window | The terminal's *input encoder* fires. The only level that can verify "what bytes does the terminal emit when key X is pressed?" |

The repo-wide taxonomy adds Browser and Real tiers; neither applies to this
package. It is maintained in `prompts/snippets/test-rigor.md` and
`.claude/skills/rust-testing/SKILL.md`; keep them in step until `md publish`
lets this page transclude it.

The harness implementations live in the shared
[`biscuit-test-harness`](../../biscuit-test-harness/README.md) crate and
include `WezTermHarness`, `KittyHarness`, `TmuxHarness`, `AppleTerminalHarness`,
and a `cliclick` helper. Tests in `cli/tests/real_terminal_render.rs` use them.
Its README documents each harness variant, when to use which, and the
environment each requires.

Skip semantics: each harness's `available()` probe checks for the required
binary on `$PATH` plus any required env (`WEZTERM_UNIX_SOCKET`,
`KITTY_LISTEN_ON`). If the host lacks the tooling, the test prints
`skipping: requires <X>` to stderr and returns `ok` — no `#[ignore]` markers,
no spurious failures.

**The cost of that convenience: a skipped test is reported as a passing test.**
`require_level!` skips by returning, which nextest cannot distinguish from a
test that ran and asserted nothing. A tier with no reachable backend is green
and fast; the same tier with a backend is green and slow. Only elapsed time
tells them apart, so **a green Level-2 run is not evidence Level 2 executed**.

Two things close this. `BISCUIT_REQUIRED_BACKENDS=tmux` makes a named backend's
absence a hard failure — prefer it over `BISCUIT_TEST_LEVEL_REQUIRED=2`, which
applies to the whole level and so panics the GUI-backed tests a headless host
legitimately cannot run. And `just test-l2` now refuses to run a package whose
`level2_*` tests exist but whose backends are all unreachable, rather than
reporting a green tier over work that never happened.

```sh
# All levels at once — Level 2 auto-skips when tooling is missing, Level 3 skips
# unless RUN_LEVEL3=1 is set
just test          # or: cargo test -p biscuit-tui -p biscuit-tui-cli

# Level 1 only — library unit tests (TestBackend, buffer asserts)
cargo test -p biscuit-tui

# Level 1 only — CLI PTY tests (manufactured input bytes via `expectrl`)
cargo test -p biscuit-tui-cli --test keyboard_protocol

# Level 2 (and Level 3 when gated on) — real terminal harness
# Auto-skips individual tests when tmux / wezterm / kitty / cliclick is missing
cargo test -p biscuit-tui-cli --test real_terminal_render

# Level 3 — OS-level keyboard injection. Focus must stay on the spawned
# terminal window during the test (cliclick on macOS, xdotool on Linux).
RUN_LEVEL3=1 cargo test -p biscuit-tui-cli --test real_terminal_render

# Run a single test by name (works at any level)
cargo test -p biscuit-tui-cli --test real_terminal_render \
    level2_tmux_ctrl_held_badge_uses_orange_bold_black_sgr -- --nocapture
```

#### Choosing the right level

| Requirement shape | Minimum level |
|---|---|
| Internal state transition | Level 1 |
| Argument parsing / output formatting | Level 1 |
| Terminal-rendered glyph / width / colour | Level 2 |
| `--json` output is valid JSON | Level 1 |
| "When the user presses X, badge Y appears" | Level 2 (kitty bytes via `wezterm cli send-text`) **or** Level 3 (real key injection) |
| "When the user holds modifier, behaviour Y" | Level 2 with kitty bytes is the most reliable; Level 3 cliclick has known macOS limitations for bare-modifier events |
| Hotkey chord triggers binding | Level 1 (manufactured bytes) + at least one Level 3 chord injection (works reliably) |
| Scrolling / overflow indicators visible | Level 2 |

#### Known limitation: cliclick + bare modifier keys on macOS

cliclick uses `CGEventCreateKeyboardEvent`, but macOS routes bare-modifier
key state through `flagsChanged` events at the AppKit layer. cliclick's
synthetic modifier events do not always reach apps via that path — the
chord case works (the modifier flag rides along with the letter
`keyDown`, which IS a normal CGEvent) but a *bare* Ctrl/Alt press
typically gets dropped before WezTerm sees it. For verifying the
"binary correctly handles bare-modifier kitty bytes" path, prefer the
**Level-2 raw-bytes test**
(`level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`) which pipes
`\x1b[57442;1u` into a real WezTerm pane via `wezterm cli send-text`.

## Documentation

For library documentation and design details, see:

- [biscuit-tui library README](../lib/README.md)
- [spec.md](../features/2026-04-16-input-tui/spec.md)
- [tech-design.md](../features/2026-04-16-input-tui/tech-design.md)

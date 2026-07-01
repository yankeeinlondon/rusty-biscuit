# Choose One

The `choose_one` component is a TUI widget that provides a single-selection list. It allows users to pick exactly one item from a set of options using a list-based interface with radio-button style indicators. Options can be arranged vertically (one per row) or horizontally (left-to-right, wrapping).

## Description

The `choose_one` component is designed for scenarios where a user must make a single choice from a predefined list. It renders each option with a selection indicator (Nerd Font `󰐱`/`󰄱` when detected, otherwise `●`/`○`) and a focus marker (`▶`) on the currently hovered row in vertical mode. In horizontal mode the active option is highlighted with a background colour instead. It supports advanced features like fuzzy filtering (search-on-type), explicit Ctrl/Alt hotkeys, and automatic scrolling.

The component is split into two parts:
- **`ChooseOne`**: A zero-sized `StatefulWidget` responsible for rendering.
- **`ChooseOneState<V>`**: The mutable state holding the options, current selection, and transient UI state (hover, scroll, filter).

`ChooseOneState::new(input)` implicitly sets `SelectionMode::Single` on the underlying `ChoiceInput<V>`. This is the default for all single-selection use cases.

## Parameters

The component is primarily configured through a `ChoiceInput<V>` struct, which is then passed to `ChooseOneState::new()`. Additional UI-only settings can be applied directly to the state.

### ChoiceInput Configuration

| Parameter | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `id` | `String` | Stable identifier for the input. | (Required) |
| `prompt` | `String` | The question or prompt text shown to the user. | (Required) |
| `help_text` | `Option<String>` | Optional help description surfaced below the prompt. When set, the text appears in a dimmed style directly under the prompt row. | `None` |
| `options` | `Vec<ChoiceOption<V>>` | The list of selectable options. | `Vec::new()` |
| `required` | `bool` | If `true`, submitting with no selection fails validation. | `false` |
| `shuffle_options` | `bool` | If `true`, randomizes option order on initialization. | `false` |
| `filter_enabled` | `bool` | Enables inline fuzzy filtering on alphanumeric input. | `false` |
| `orientation` | `Orientation` | Layout direction (`Vertical` or `Horizontal`). | `Vertical` |
| `sort` | `Option<SortOrder>` | Optional ordering applied before state construction. | `None` |

### ChooseOneState Extensions

| Method | Description |
| :--- | :--- |
| `with_label(Label)` | Attaches a label (prompt) rendered relative to the list. |
| `with_theme(ComponentTheme)` | Overrides the default visual styling. |
| `with_key_bindings(KeyBindings)` | Overrides the default key mapping. |
| `with_initial_selection(&str)` | Pre-selects an option by its stable `id`. |
| `with_initial_value(&str)` | Pre-selects an option by matching its `value` field using `PartialEq`. When no `--delimiter` is used, the `value` equals the `label`, so both methods behave similarly. |

### Key Bindings (Default)

- **`Space`**: Selects the currently hovered item (without submitting).
- **`Enter`**: Selects the currently hovered enabled item and submits.
- **`Esc`**: Restores the initial selection and submits. (If filtering, first `Esc` closes the filter; second `Esc` restores and submits).
- **`Up` / `k`**: Moves the hover cursor up (vertical) or to the closest column in the row above (horizontal).
- **`Down` / `j`**: Moves the hover cursor down (vertical) or to the closest column in the row below (horizontal).
- **`Left` / `h`**: Moves to the previous option.
- **`Right` / `l`**: Moves to the next option.
- **`Home` / `g`**: Jumps to the first enabled option.
- **`End` / `G`**: Jumps to the last enabled option.
- **`Ctrl+<char>`**: Selects the option with the matching explicit `Ctrl` hotkey and submits.
- **`Alt+<char>`**: Selects the option with the matching explicit `Alt` hotkey and submits.
- **`Alphanumeric`**: If `filter_enabled` is true, starts a fuzzy search. Otherwise, the keystroke is ignored.

## Behavioral Notes

- **Enter Behavior**: `Enter` always selects the currently hovered enabled item and submits. There is no automatic selection of the hovered item on submit; the user must explicitly select with `Space` or `Enter`.
- **Esc Behavior**: `Esc` restores the selection to whatever it was when the component started (the `initial_selected` value) and then submits with exit code `0`. If the user navigated or changed the selection with `Space`, those changes are discarded. This makes `Esc` a "reset and submit" action, not a cancel.
- **Fuzzy Filtering**: When active, only options matching the pattern are displayed. The hover cursor is snapped to the first visible result, and matching characters are highlighted in the labels.
- **Explicit Hotkeys**: Options can carry explicit `Ctrl` or `Alt` hotkeys (e.g., `[CTRL+R]`). These are parsed from option text and select + submit when pressed.
- **Disabled Options**: Options can be marked as `disabled`. They are rendered dimmed, cannot be hovered or selected, and are skipped by navigation.

## Keyboard Protocol & Hotkey Badges

The `choose_one` runner attempts to enable the Kitty keyboard protocol on startup so that bare `Ctrl` and `Alt` presses are reported as distinct events. When this succeeds, holding `Ctrl` or `Alt` alone immediately surfaces coloured hotkey badges next to any option that carries a matching shortcut (e.g., `[CTRL+R]`). The badges disappear when the modifier is released.

### Terminal Compatibility

| Terminal | Bare Modifier Badges |
| :--- | :--- |
| Kitty | Yes |
| WezTerm | Yes |
| Ghostty | Yes |
| foot | Yes |
| Alacritty (≥ 0.13) | Yes |
| iTerm2 (modern) | Yes |
| Older terminals / Windows CMD | No — badges only flash on chord press (e.g. `Ctrl+R`) |

On terminals that do not emit modifier-only key events, the component falls back to a deadline-based approach: a chord press (e.g., `Ctrl+R`) arms a short timer (~300 ms) during which the badges remain visible, giving the user a brief window to read the shortcuts before they fade.

### Production-Readiness Scope (Bare-Modifier Badges)

The bare-modifier badge UX above is verified as follows:

- **Ctrl and Alt chord presses** (`Ctrl+R`, `Alt+R`) — verified end-to-end by feeding the exact terminal byte sequences a chord emits (the `0x12` control byte and `ESC r`) into a headless tmux pane and asserting the option is selected and submitted. Each modifier has its own Level-2 test (`level2_tmux_ctrl_r_chord_selects_red`, `level2_tmux_alt_r_chord_selects_red`). These run in the background and never steal desktop focus.
- **Bare-modifier badge toggles** (e.g. holding `Alt` or `Ctrl` alone, or `Alt+Space` / `Ctrl+Space` sticky toggles) — the binary's handler is verified, but the OS → terminal handoff is **not** automatically verified on macOS for the `flagsChanged` reason below.
- **The binary's internal handler for the kitty bare-modifier escape `\x1b[57442;1u`** — verified at Level 2 by piping the literal bytes through `wezterm cli send-text` and asserting the badge appears (`level2_wezterm_bare_ctrl_kitty_bytes_reveal_badges`).
- **End-to-end "user holds bare Ctrl in WezTerm → badge appears"** — **not** automatically verified on macOS. No userspace path on macOS can synthesise the `flagsChanged` event that AppKit apps (including WezTerm) observe for bare-modifier presses, so this handoff cannot be exercised by automated keyboard injection without a focused GUI window — which steals desktop focus and yields flaky results. We deliberately do not pay that cost: whether a physical key makes a terminal emit a given byte is the terminal's input encoder, not biscuit-tui's code. Chord cases work because the modifier flag rides along with the letter keyDown event as a normal CGEvent, which is why their byte sequences are the ones we inject above.

Treat bare-modifier visibility as **best-effort** on macOS: it works in practice on all terminals listed in the table above when their bare-modifier reporting is correctly configured (e.g. `enable_kitty_keyboard = true` in `wezterm.lua`). The byte-level contracts that are biscuit-tui's own responsibility are fully verified at Level 2; the OS → terminal physical-key encoder is not, by design. Chord-based shortcuts remain the supported, fully-verified interaction.

## Helper Functions

The `biscuit_tui::helpers::choice_builders` module provides convenience functions for constructing `ChoiceInput<String>` from common sources:

- `choose_one_from_csv(id, prompt, csv)` — builds options from a comma-separated string.
- `choose_one_from_markdown_list(id, prompt, markdown)` — builds options from a Markdown bullet or numbered list.
- `choose_one_from_dictionary(id, prompt, yaml_or_json)` — builds options from a YAML/JSON mapping where keys become labels and values become option values.

See the [CLI Reference](../cli-reference.md) and [Theming & Configuration](../theming.md) docs for cross-cutting topics.

## Usage Examples

### 1. Basic Single-Select (Strings)
A simple list of options using the default `String` value type.

```rust
use biscuit_tui::components::choose::{ChoiceInput, ChoiceOption};
use biscuit_tui::components::choose_one::ChooseOneState;

let input = ChoiceInput::new("color", "Pick a color")
    .with_options(vec![
        ChoiceOption::new("r", "Red", "red"),
        ChoiceOption::new("g", "Green", "green"),
        ChoiceOption::new("b", "Blue", "blue"),
    ])
    .required();

let mut state = ChooseOneState::new(input);
```

### 2. Typed Values and Pre-selection
Using a custom enum for values and setting an initial choice.

```rust
#[derive(Clone, PartialEq)]
enum Mode { Fast, Safe, Deep }

let input = ChoiceInput::new("mode", "Select analysis mode")
    .with_options(vec![
        ChoiceOption::new("f", "Fast", Mode::Fast),
        ChoiceOption::new("s", "Safe", Mode::Safe),
        ChoiceOption::new("d", "Deep", Mode::Deep),
    ]);

let mut state = ChooseOneState::new(input)
    .with_initial_selection("s"); // Pre-selects "Safe"
```

### 3. Rendering in a Widget
Implementing the render logic within a Ratatui `render` loop.

```rust
fn render(area: Rect, buf: &mut Buffer, state: &mut MyState) {
    let widget = ChooseOne::new();
    // state.choose_one is a ChooseOneState<V>
    ratatui::widgets::StatefulWidget::render(widget, area, buf, &mut state.choose_one);
}
```

## CLI Usage

The `choose_one` component is exposed via the `question choose-one` command. It writes the selected value to STDOUT.

### Common Flags

**Option sources (mutually exclusive):**
- Positional arguments — `question choose-one Apple Banana Cherry`
- `--csv <TEXT>` — comma-separated list
- `--list <TEXT>` — newline-separated list
- `--rows <TEXT>` — newline-separated `label::value` pairs
- `--file <PATH>` — JSON, JSONL, NDJSON, YAML, TOML, or CSV file containing an array
- `--md <PATH> <PROP>` — YAML frontmatter array property from a Markdown file
- `--options <TEXT>` — hidden alias for `--csv` (backward compatibility)
- Piped stdin (automatic when stdin is not a TTY)

**TOML note.** Standard TOML cannot represent a top-level bare array (the
document root must be a table), so a TOML options file **must** use the
`options = [...]` table form. Entries may be strings,
inline tables (`options = [{ label = "Red", value = "apple" }]`), or
array-of-tables records (`[[options]]`) with `label`, `value`, `hotkey`, and
`disabled` fields. Files with any other top-level key (e.g. `colors = [...]`)
fail with `option file must contain an array`.

**Selection & filtering:**
- `--selected <VALUE>`: Pre-select a specific value.
- `--required`: Fail if no item is selected.
- `--delimiter <CHAR>`: Split each option string into `label<CHAR>value`.
- `--no-filter`: Disable fuzzy search (use hotkey shortcuts instead).
- `--sort <natural|inverse|asc|desc>`: Reorder options before display. `reverse` is a hidden alias for `inverse`.

**Hotkeys & normalization:**
- `--numeric-hot-keys`: Auto-assign Ctrl+1..9,0 then Alt+1..9,0 to the first 20 options.
- `--label-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>`: Transform option labels.
- `--value-convention <caps|lowercase|camel-case|pascal-case|kebab-case|snake-case|title-case>`: Transform option values.
- `::` delimiter in option text splits `label::value` (takes precedence over conventions).
- `[CTRL+X]`, `[ALT+X]`, `[OPT+X]` prefixes in option text assign explicit hotkeys.
- `--hotkey-badges <auto|always|never>`: When to render coloured hotkey badges next to options that carry an explicit `Ctrl+X`/`Alt+X` shortcut. `auto` (the default) shows badges while the matching modifier is held — using a brief deadline-based fallback (~300 ms) on terminals that do not emit modifier-only key events. `always` forces Ctrl badges on for the lifetime of the prompt; `never` hides them entirely. Vertical layouts render badges inline immediately after the option label; horizontal layouts drop them on the row directly below the option (Up/Down navigation skips that sub-row).

**Chrome:**
- `--border`, `--border-label <TEXT>`, `--border-style <STYLE>`: Border chrome.
- `--margin <N>`, `--mt <N>`, `--mb <N>`, `--ml <N>`, `--mr <N>`: Outer margin.
- `--padding <N>` / `-p <N>`, `--pt <N>`, `--pb <N>`, `--pl <N>`, `--pr <N>`: Inner padding.
- `--active-color <grey|green|yellow|red>`: Background colour for the actively hovered option (default `grey`). The renderer combines this with the detected terminal background to pick a foreground that meets the spec's contrast rule (white text on dark/unknown, black text on light). The active highlight covers only the focus indicator + selection glyph + label + one trailing blank cell — never the full row width.

### Global Flags

- `--output <raw|json|null>`: Serialisation format for the submitted value (`raw` is the default). `null` emits the value followed by a NUL (`\0`) terminator instead of a newline.
- `--height <CELLS_OR_PERCENT>`: Render inline at an explicit height instead of fullscreen.

### Exit Codes

| Code | Meaning |
| :--- | :--- |
| `0` | Value submitted successfully (including `Esc`, which restores the initial selection). |
| `130` | User pressed `Ctrl-C` (SIGINT). |

### Positional vs `--csv`

Positional arguments are the modern default. `--options` is a hidden backward-compatibility alias for `--csv`.

```bash
# Positional args (preferred)
question choose-one Apple Banana Cherry

# Comma-separated flag
question choose-one --csv "Apple,Banana,Cherry"
```

### Example CLI Commands

```bash
# Select a server from a list
question choose-one \
  --label "Target Server" \
  --delimiter ":" \
  "Production:prod-01" \
  "Staging:stg-01" \
  "Development:dev-01"

# Fuzzy search with inverse sort and numeric hotkeys
question choose-one --csv "Apple,Banana,Cherry" --sort inverse --numeric-hot-keys

# Load options from a file with padding
question choose-one --file options.json --padding 2 --border

# Label::value pairs with convention transforms
question choose-one --rows $'Red::apple\nGreen::pear' --label-convention title-case
```

## Enhancement Suggestions

1. **Category Grouping**: Add support for section headers or categories within the list to organize large sets of options.
2. **Sub-labels/Descriptions**: Support secondary hint text for each option, displayed in a dimmed style below or beside the main label.
3. **Multi-column Layout**: Support rendering options in multiple columns for long lists with short labels to optimize terminal screen real estate.
4. **"None" Option**: Add a built-in "clear selection" or "(none)" option for non-required inputs to make unselecting easier.

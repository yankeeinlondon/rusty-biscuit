The `question` CLI (aka, the CLI from biscuit-tui) is a little to basic and not ergonomic enough to use.
In this feature we will improve the `choose-one` and `choose-many` commands.

## STDIN

Both `choose-one` and `choose-many` should be able to read from STDIN. As an example:

```sh
printf "%s\n" "uno" "dos" "tres" | question choose-one
```

This should pass the three values "uno", "dos", and "tres" in as three values to choose from. By default, the "label" and the "value" are the same (e.g., "uno" is shown and if the user selects "uno" then "uno" is the value returned). See [Label/Value Separation](#labelvalue-separation) for how to distinguish them.

## Positional Params

Both `choose-one` and `choose-many` should take all positional arguments as choices:

```sh
question choose-one "uno" "dos" "tres"
```

Just like the STDIN example, in this example we'll present "uno" "dos" and "tres" as three options for the user to choose from.

## Label/Value Separation

The tool supports separating the display text (Label) from the underlying data (Value) using the `--delimiter` switch.

- **Delimiter**: Add a `--delimiter <char>` CLI switch.
- **Behavior**: When STDIN or Positional Arguments are provided and `--delimiter` is set, the input string is split on the **first occurrence** of the delimiter.
    - The first part is the **Label** (displayed to the user in the TUI).
    - The second part is the **Value** (returned to the user upon submission).
    - If the delimiter is not found in the string, it defaults to **Label = Value**.

Example:
```sh
question choose-one --delimiter ":" "Option A:id_1" "Option B:id_2"
```
In this case, the user sees "Option A" and "Option B", but "id_1" or "id_2" is returned.

## Default Selected

Both `choose-one` and `choose-many` will take a value based parameter of `selected={value}` where `{value}` should be a valid id for one of the options in the list.

- If the value passed in is not a valid id in the list then it is ignore.
- If the value passed in is a valid id then when the question is first presented the selected item (for `choose_one`) or selected items (for `choose-many`) will be selected and be visually distinguished from the other items.

## Interaction

- **Navigation**: The user uses the `Up` and `Down` arrow keys to change which of the choices is "active". It is visually clear to the user at all times which line is "active".
- **Selection**: Pressing the `Space` bar toggles the active item's selection state (selected/not-selected).
    - *Note: with the `choose-one` command, selecting one item also unselects the previously selected.*
- **Shortcuts (choose-many only)**:
    - `Ctrl+A`: Select all items.
    - `Ctrl+D`: Deselect all items.
- **Submission**: Pressing `Enter` submits the current selection and exits the tool.
- **Fallback Submission**: If `Enter` is pressed and NO items have been explicitly selected via `Space`, the tool will gracefully fallback to submitting the currently "active" item.
- **Cancellation**:
    - `Ctrl+C` (SIGINT): Exits the process with code `130` and produces no output.
    - `ESC`: Exits the process with code `1` and produces no output.

## Output

- **choose-many**: The selected values are output to `STDOUT` newline-separated (one value per line).
- **choose-one**: The selected value is output to `STDOUT`.

## Search/Filtering

The tool includes **Fuzzy Search** functionality (similar to `fzf`).

- **Initial State**: The search prompt is **hidden by default** when the tool starts.
- **Activation**: The search prompt automatically appears and begins filtering the list as soon as the user starts typing any alphanumeric characters.
- **Filtering**: As the user types, the list is dynamically filtered using a fuzzy matching algorithm against the item labels.

## CLI Switches

Both of the _choose_ commands will include the following CLI switches:

- `--delimiter <char>` sets the character used to split input into Label and Value (defaults to no split).
- `--border` adds a line border (inspired by fzf)
- `--border-label <title>` adds a text label to the border line (inspired by fzf)
- `--border-style <style>` sets the style of the border with one of the enumerated values:
    - rounded
    - sharp
    - bold
    - double
    - block
    - thinblock
    - horizontal
    - vertical
    - line
    - top
    - bottom
    - left
    - right
    - none

    This too is inspired by `fzf`

- '--margin <#>' provides a top,bottom,left,right margin of some number of characters
- `--mb <#>`, `--mt <#>`, `--ml <#>`, `--mr <#>` discrete margin settings
- `--height <# | %>` takes either an integer number or a percentage of the overall viewport height of the terminal
- `--sort <sort>`
    - reverse - _reverses the provided natural order_
    - asc - _lists labels in ascending order_
    - desc - _lists labels in a decencding order_

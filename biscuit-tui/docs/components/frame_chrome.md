# FrameChrome

The `FrameChrome` is a stateful wrapper widget that draws an optional border, title, and margin around any inner component. It is the mechanism by which the CLI's `--border`, `--border-label`, `--border-style`, and `--margin` flags are rendered.

## Description

`FrameChrome<'a, W>` is a generic `StatefulWidget` that wraps an inner widget `W`. On render, it first shrinks the allocated area by the margin, then (if a visible border is configured) draws a ratatui `Block` with the requested sides and style, then applies interior padding, and finally renders the inner widget into the remaining interior rectangle.

It is not a user-input component itself — it has no `HandleEvent` implementation and no value. It is a **container** that adds visual chrome around a real input widget.

The companion `FrameChromeConfig` struct is a plain data configuration object used by the CLI layer to assemble a `FrameChrome` from parsed flags.

## Parameters & Defaults

### FrameChromeConfig

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `border` | `BorderStyle` | `BorderStyle::None` | Which sides and glyph style to draw. |
| `border_label` | `Option<String>` | `None` | Title rendered in the top-left of the border. |
| `bottom_label` | `Option<String>` | `None` | Title rendered in the bottom-left of the border. Used by the standalone runner to inline the help-hint footer inside the bottom border so the corner glyphs survive. Silently ignored when the resolved border has no `BOTTOM` segment. |
| `margin` | `Margin` | `Margin::default()` (all zeros) | Cells of margin on each side outside the border. |
| `padding` | `Padding` | `Padding::uniform(1)` | Cells of padding on each side inside the border. |
| `border_style` | `Style` | `Style::default()` | ratatui style applied to the border glyphs. |
| `show_on_exit` | `bool` | `false` | When `false` (default) the standalone runner clears the inline viewport on exit (fzf-style); when `true` the final frame is preserved and the cursor moves to the row just below the chrome. |

### BorderStyle Variants

| Variant | Sides | Glyph Style |
|---------|-------|-------------|
| `None` | — | No border drawn |
| `Rounded` | All four | Rounded corners |
| `Sharp` | All four | Plain single-line |
| `Bold` | All four | Thick single-line |
| `Double` | All four | Double-line |
| `Block` | All four | Quadrant-outside |
| `ThinBlock` | All four | Quadrant-inside |
| `Horizontal` | Top + Bottom | Plain |
| `Vertical` | Left + Right | Plain |
| `Line` | Top only | Plain |
| `Top` | Top only | Plain |
| `Bottom` | Bottom only | Plain |
| `Left` | Left only | Plain |
| `Right` | Right only | Plain |

### Margin

Four-sided margin with per-side overrides. Margins are applied **outside** the border — they shrink the area before the border is drawn.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `top` | `u16` | `0` | Cells of margin above |
| `bottom` | `u16` | `0` | Cells of margin below |
| `left` | `u16` | `0` | Cells of margin to the left |
| `right` | `u16` | `0` | Cells of margin to the right |

Use `Margin::uniform(n)` for equal sides, then override individual fields as needed.

### Padding

Four-sided padding with per-side overrides. Padding is applied **inside** the border — it shrinks the area available to the inner widget after the border is drawn. `Padding::default()` is `Padding::uniform(1)` (matching the spec's library-level default) so widgets do not visually touch the border.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `top` | `u16` | `1` | Cells of padding above |
| `bottom` | `u16` | `1` | Cells of padding below |
| `left` | `u16` | `1` | Cells of padding to the left |
| `right` | `u16` | `1` | Cells of padding to the right |

Constructors:

- `Padding::default()` — `1` cell on every side (the library default).
- `Padding::uniform(n)` — `n` cells on every side. Override individual fields after construction as needed.
- `Padding::zero()` — `0` cells on every side. Use this to opt out of interior spacing entirely.
- `Padding::none()` — alias for `Padding::zero()`. Reads naturally at call sites that want "no padding".

Note: a `FrameChromeConfig` built from `Default::default()` is **not** empty — its padding alone (`uniform(1)`) affects layout. `FrameChromeConfig::is_empty()` only returns `true` when padding is `Padding::zero()` and there is no border or margin.

## Usage Examples

### 1. Wrapping a Component with a Border

```rust
use tui_chrome::prelude::*;
use tui_chrome::core::{BorderStyle, FrameChrome, FrameChromeConfig, Margin};

let config = FrameChromeConfig {
    border: BorderStyle::Rounded,
    border_label: Some("Pick a color".into()),
    ..Default::default()
};
let frame = FrameChrome::from_config(ChooseOne::new(), &config);
```

### 2. Border with Margin

```rust
let config = FrameChromeConfig {
    border: BorderStyle::Double,
    border_label: Some("Settings".into()),
    margin: Margin::uniform(1),
    border_style: Style::default().fg(Color::Cyan),
};
let frame = FrameChrome::from_config(BooleanSwitch::new(), &config);
```

### 3. No-Op Wrapper

When `FrameChromeConfig::is_empty()` returns `true` (no visible border, zero margin), callers can skip wrapping entirely or use `FrameChrome::bare(inner)` for type uniformity.

```rust
let frame = FrameChrome::bare(TextInput::new());
```

See the [CLI Reference](../cli-reference.md) for global flags and exit codes, and [Theming & Configuration](../theming.md) for shared visual settings.

## CLI Usage

The `FrameChrome` is not exposed as its own subcommand. Instead, it is implicitly created by the `question` CLI for the `choose-one` and `choose-many` subcommands (and any other command that flattens `ChooseChromeArgs`).

### Common Flags

- `--border`: Draw a border (defaults to `Rounded` style).
- `--border-label <TEXT>`: Title in the top-left of the border. Implies `--border`.
- `--border-style <STYLE>`: Border glyph style (see `BorderStyle` variants above). Any non-`none` value implies `--border`. Explicit `none` suppresses the border even if `--border` is set.
- `--margin <CELLS>`: Uniform margin on all four sides.
- `--mt`, `--mb`, `--ml`, `--mr`: Per-side margin overrides that take precedence over `--margin`.
- `--padding <CELLS>` / `-p <CELLS>`: Uniform padding on all four sides inside the border.
- `--pt`, `--pb`, `--pl`, `--pr`: Per-side padding overrides that take precedence over `--padding`.

### Example CLI Commands

```bash
# Rounded border with a title
question choose-one --border --border-label "Server" Alpha Beta Gamma

# Double border with 1-cell margin
question choose-many --border-style double --margin 1 Red Green Blue

# Thick border with custom margin and padding
question choose-one --border-style bold --margin 2 --mt 0 --padding 1 One Two Three

# No border, only padding
question choose-many --border-style none --padding 2 Red Green Blue
```

## Enhancement Suggestions

1. **Custom Title Position**: Support rendering the border label at different positions (center, right) via a `title_position` field on `FrameChromeConfig`.
2. **Footer/Subtitle**: Add a second label rendered at the bottom of the border for status text or contextual hints.
3. **Styled Title**: Allow the border label to have its own `Style` independent of the border glyph style, so titles can be highlighted without affecting the border color.

# Boolean Switch

The `boolean_switch` is a TUI component that provides a simple and intuitive toggle interface for boolean values. It renders a "track" with labels representing the "ON" and "OFF" states, and a thumb indicator showing the current selection.

## Description

The `boolean_switch` is designed for binary choices where a clear visual indication of the current state is required. It is commonly used for settings, feature flags, or any simple "yes/no" selection. The component supports both toggling and direct "force" switching using directional keys, making it robust for various user interaction patterns.

## Parameters

The component's state and appearance are managed via `BooleanSwitchState`.

| Parameter | Type | Description | Default |
| :--- | :--- | :--- | :--- |
| `checked` | `bool` | The current state of the switch. | `false` |
| `label` | `Option<Label>` | An optional descriptive label for the component. | `None` |
| `on_label` | `String` | Text displayed for the "active" (true) state. | `"ON"` |
| `off_label` | `String` | Text displayed for the "inactive" (false) state. | `"OFF"` |
| `theme` | `ComponentTheme` | Visual styling, including colors and the thumb character. | `ComponentTheme::default()` |
| `bindings` | `KeyBindings` | Mapping of keys to actions for the component. | Standard defaults |

### Key Bindings (Default)

- **`Space`**: Toggles the current state.
- **`Left` / `h`**: Forces the switch to the **OFF** position.
- **`Right` / `l`**: Forces the switch to the **ON** position.
- **`Enter`**: Submits the current value and exits.
- **`Esc`**: Cancels the interaction without submitting.

## Behavioral Notes

- **Validation**: Since a boolean choice is always valid (it's either true or false), the component does not implement complex validation logic and is always in a "valid" state.
- **Standalone Widget**: The `BooleanSwitch` struct is zero-sized; all mutable state, including the current value and labels, is stored in the `BooleanSwitchState`.
- **Label Positioning**: Labels can be positioned `Above`, `Below`, `Left`, or `Right` relative to the switch track using the `Label` configuration.

## Usage Examples

### 1. Basic Toggle Switch
A simple switch with default "ON" and "OFF" labels.

```rust
let mut state = BooleanSwitchState::new();
// Rendered: [●OFF | ON ]
```

### 2. Custom Labels and Initial State
Configuring a switch with custom text, an initial `true` value, and a label positioned above.

```rust
let mut state = BooleanSwitchState::new()
    .with_labels("Enabled", "Disabled")
    .with_value(true)
    .with_label(Label::new("Network Connection", LabelPosition::Above));

// Rendered:
// Network Connection
// [ Disabled | ●Enabled]
```

### 3. Integration in a Ratatui Render Loop
Using the component within a standard Ratatui application.

```rust
impl StatefulWidget for MyView {
    type State = MyViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let switch = BooleanSwitch::new();
        // Assuming switch_state is part of MyViewState
        StatefulWidget::render(switch, area, buf, &mut state.switch_state);
    }
}
```

## CLI Usage

The `boolean_switch` is exposed as a standalone command via the CLI (e.g., `question boolean-switch`). This allows it to be used in shell scripts to capture user input.

### Common Flags

- `--label <TEXT>`: The text to display as a label.
- `--label-position <Above|Below|Left|Right>`: Where to render the label relative to the switch.
- `--initial <true|false>`: The starting state of the switch.
- `--labels <ON,OFF>`: A comma-separated pair of custom labels for the on and off states.

### Example CLI Command

```bash
# Ask the user if they want to enable telemetry
question boolean-switch --label "Enable Telemetry?" --labels "YES,NO" --initial true
```

## Enhancement Suggestions

1. **Mouse Support**: Implement hit-testing to allow users to toggle the switch or click specific labels using the mouse.
2. **Indeterminate State**: Add support for a "mixed" or "tri-state" mode, useful for representing values that are partially set or unknown.
3. **Prefix/Suffix Icons**: Allow the integration of icons (e.g., from Nerd Fonts) next to the "ON" and "OFF" labels to provide better visual cues (e.g., a checkmark and a cross).
4. **Animation Support**: Add support for a transition effect when the thumb moves between the "ON" and "OFF" positions for a more polished feel in terminals that support it.

# Mermaid Errors — Block Style Error Ideas

## Error Type: `MermaidThemeError`

**Source:** `darkmatter/lib/src/mermaid/theme.rs:14`

This error is produced when parsing or validating a custom `MermaidTheme` from JSON input. Themes are used to style Mermaid diagrams rendered to the terminal. The error has two variants:

| Variant | Current message | Trigger |
|---------|----------------|---------|
| `InvalidJson` | `"Invalid JSON: {0}"` | Malformed JSON passed to `TryFrom<&str>`, `TryFrom<String>`, or `TryFrom<serde_json::Value>` |
| `InvalidColor` | `"Invalid color value for '{field}': {value}"` | A theme field contains a color string that fails validation |

Both variants currently produce flat single-line error messages. The `InvalidJson` variant wraps a `serde_json::Error` via `#[from]`, which gives decent line/column information but is not styled for terminal readability. The `InvalidColor` variant names the field and value but offers no guidance on accepted formats.

---

## Variant: `InvalidJson`

The `InvalidJson` variant is auto-converted from `serde_json::Error`. It fires when the JSON string cannot be parsed into a `MermaidTheme` struct. Common causes include missing quotes, trailing commas, or a value with the wrong type (e.g., passing a number where a string is expected).

### Idea 1: Show the malformed JSON with an inline pointer

Use a `Status` header (Failure state, bold red "MermaidThemeError:" prefix) followed by a `StatusBlock` whose body contains a code block showing the problematic JSON fragment and a pointer to the parse error location from `serde_json::Error`.

**Rendered example:**

```
⤫ MermaidThemeError: Invalid Mermaid theme JSON

┃ The theme JSON could not be parsed.
┃
┃   { "background": "#1e1e1e", "primaryColor":  }
┃                                       ^ expected a string value
┃
┃ Hint: All color values must be quoted strings. Check for trailing commas,
┃ missing quotes, or unquoted values.
```

**Components:**
- `Status::from_prose("MermaidThemeError: Invalid Mermaid theme JSON").state(StatusState::Failure)` — bold red "MermaidThemeError:" with bold title
- `StatusBlock::new(StatusState::Failure)` with:
  - `.body()` containing a `Prose` with the explanatory text and the code block showing the invalid JSON
  - `.hint("All color values must be quoted strings. Check for trailing commas, missing quotes, or unquoted values.")` 

### Idea 2: List the closest valid field names on type-mismatch errors

When `serde_json` reports an unknown field (e.g., `"primayColor"` instead of `"primaryColor"`), use Levenshtein distance to suggest the closest match. Display inside a `StatusBlock`.

**Rendered example:**

```
⤫ MermaidThemeError: Unknown field in theme JSON

┃ The field "primayColor" is not recognized.
┃
┃   Did you mean "primaryColor"?
┃
┃ Valid fields include: background, primaryColor, primaryTextColor,
┃ primaryBorderColor, secondaryColor, lineColor, textColor, ...
┃
┃ Hint: Theme fields use camelCase. See the MermaidTheme documentation for
┃ the full list of supported fields.
```

**Components:**
- `Status::from_prose("MermaidThemeError: Unknown field in theme JSON").state(StatusState::Failure)`
- `StatusBlock::new(StatusState::Failure)` with:
  - `.body()` with `Prose` showing the unknown field, suggestion, and a wrapped list of valid field names
  - `.hint("Theme fields use camelCase. See the MermaidTheme documentation for the full list of supported fields.")`

---

## Variant: `InvalidColor`

The `InvalidColor` variant carries the `field` name and the offending `value`. It fires during post-parse validation when a color string does not match an expected format (hex, named color, etc.).

### Idea 1: Show accepted color formats with examples

A `StatusBlock` that names the field, shows the bad value, and lists the accepted color formats with inline examples.

**Rendered example:**

```
⤫ MermaidThemeError: Invalid color value

┃ The field "primaryColor" has an invalid color value: "neon-blue"
┃
┃ Accepted color formats:
┃
┃   Hex (6-digit)    #569cd6
┃   Hex (3-digit)    #eee
┃   Hex (8-digit)    #569cd6ff
┃   Named CSS color  steelblue
┃   Transparent      transparent
┃
┃ Hint: Use a hex color like "#569cd6" or a named CSS color. The "transparent"
┃ keyword is also accepted for background fields.
```

**Components:**
- `Status::from_prose("MermaidThemeError: Invalid color value").state(StatusState::Failure)`
- `StatusBlock::new(StatusState::Failure)` with:
  - `.body()` using `Prose` for the field name/value pair followed by a formatted list of accepted formats
  - `.hint("Use a hex color like \"#569cd6\" or a named CSS color. The \"transparent\" keyword is also accepted for background fields.")`

### Idea 2: Highlight the field within a full theme JSON snippet

Show a truncated version of the theme JSON with the offending field visually highlighted (using `Prose` bold/red styling), so the user can see the context of the error.

**Rendered example:**

```
⤫ MermaidThemeError: Invalid color value for "lineColor"

┃ The color "rgb(300, 0, 0)" in field "lineColor" is not a valid color.
┃
┃   {
┃     "background": "#1e1e1e",
┃     "primaryColor": "#569cd6",
┃     "lineColor": "rgb(300, 0, 0)"  ← invalid
┃   }
┃
┃ Hint: RGB/RGBA function syntax is not supported. Use hex colors like
┃ "#ff0000" or named CSS colors instead.
```

**Components:**
- `Status::from_prose("MermaidThemeError: Invalid color value for \"lineColor\"").state(StatusState::Failure)`
- `StatusBlock::new(StatusState::Failure)` with:
  - `.body()` containing `Prose` text that includes a code-fenced JSON snippet where the invalid field is annotated with an `← invalid` marker
  - `.hint("RGB/RGBA function syntax is not supported. Use hex colors like \"#ff0000\" or named CSS colors instead.")`

---

## Implementation Notes

- Both variants should implement the `BlockError` trait (proposed in `spec.md`), returning a `StatusBlock` configured with `StatusState::Failure`.
- The `InvalidJson` variant can extract line/column from `serde_json::Error` via its `line()` and `column()` methods to build the JSON snippet pointer.
- The `InvalidColor` variant already has structured data (`field`, `value`) — the `BlockError` implementation should embed these into the `Prose` body using `{{bold}}` for field names and `{{red}}` for invalid values.
- The `.hint()` method on `StatusBlock` is ideal for the one-line actionable guidance shown at the bottom of each block.

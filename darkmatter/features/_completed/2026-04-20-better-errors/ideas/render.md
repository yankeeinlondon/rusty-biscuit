# Render Error Ideas

Block-style error improvement proposals for the three error enums in the `render` module.

## StylesheetError

**File:** `darkmatter/lib/src/render/stylesheet.rs:18`

8 variants covering CSS declaration parsing and value validation.

| Variant | Fields | Current message |
|---------|--------|-----------------|
| `InvalidDeclaration` | `declaration` | `invalid declaration '{declaration}'; expected 'property: value'` |
| `InvalidPropertyName` | `name` | `invalid CSS property name '{name}'` |
| `PropertyValueTypeMismatch` | `property`, `expected`, `actual`, `value` | `property '{property}' expects '{expected}' values, but got '{actual}' from '{value}'` |
| `InvalidSizing` | `value` | `invalid CSS sizing value '{value}'` |
| `InvalidSizingMulti` | `value` | `invalid CSS multi-sizing value '{value}'; expected 1 to 4 sizing tokens` |
| `InvalidColor` | `value` | `invalid CSS color value '{value}'` |
| `InvalidInteger` | `value` | `invalid integer value '{value}'` |
| `InvalidRawValue` | `value` | `invalid raw CSS value '{value}'` |
| `InvalidCustomProperty` | `name` | `invalid custom CSS property '{name}'` |

### Key variant: `PropertyValueTypeMismatch`

This variant already carries rich structured data (`property`, `expected`, `actual`, `value`) but the flat message loses the visual hierarchy that makes the mismatch immediately obvious.

**Block-style proposal:**

```
StylesheetError: value type mismatch for 'margin-top'
┃ property 'margin-top' expects sizing values but got 'color' from 'red'
┃
┃ expected: sizing (e.g. 12px, 1rem, auto, 0)
┃ received: red
┃
┃ hint: 'margin-top' accepts a single sizing token.
┃   Try: margin-top: 8px
```

**Implementation notes:**

- Use `Status::new(StatusState::Failure)` for the title line: bold-red `StylesheetError:` + bold `value type mismatch for 'margin-top'`
- Use `StatusBlock::new(StatusState::Failure)` with:
  - `.header("...")` via `Prose` tokens for the property detail line
  - `.body(...)` showing expected vs received with the `CssValueKind` `Display` impl and a concrete example list derived from `CssProp::expected_kind()`
  - `.hint(...)` with a suggested corrected declaration

The body could be built from a `BlockQuote` containing a small `Prose` snippet:

```rust
let body = format!(
    "{{dim}}expected:{{reset}} {} (e.g. {})\n{{dim}}received:{{reset}} {}",
    expected,
    sizing_examples_for_property(&property),
    value
);
let block = StatusBlock::new(StatusState::Failure)
    .header(format!(
        "{{bold}}property '{{reset}}{{bold}}{{rgb 97,175,239}}{}{{reset}}{{bold}}' expects {} values but got {} from '{{reset}}{{bold}}{{rgb 152,195,121}}{}{{reset}}{{bold}}'{{reset}}",
        property, expected, actual, value
    ))
    .body(RenderableContent::String(body))
    .hint(format!("hint: '{}' accepts a single sizing token.\n  Try: {}: {}", property, property, suggested_value));
```

### Key variant: `InvalidColor`

Color values are richly varied (hex, rgb, rgba, named, expressions) and a bare `invalid CSS color value 'xyz'` gives no guidance.

**Block-style proposal:**

```
StylesheetError: invalid CSS color value
┃ the value 'rgb(300, 0, 0)' could not be parsed as a color
┃
┃ accepted formats:
┃   hex: #fff  #ff00aa  #ffffff80
┃   rgb: rgb(255, 0, 0)
┃   rgba: rgba(255, 0, 0, 0.5)
┃   named: red  cornflower-blue  transparent
┃   expression: var(--my-color)
```

**Implementation notes:**

- `Status` title: bold-red error name + `invalid CSS color value`
- `StatusBlock` body: a `Prose`-formatted list of accepted color formats, using the same color-coding as `Stylesheet::to_terminal_string()` (RGB `152,195,121` for color values)
- The hint section could detect common mistakes (e.g., channel overflow in `rgb()`, missing `#` prefix on hex, underscores in names) and offer a targeted suggestion

---

## LinkError

**File:** `darkmatter/lib/src/render/link.rs:32`

7 variants covering hyperlink construction, parsing from HTML/Markdown, and attribute validation.

| Variant | Fields | Current message |
|---------|--------|-----------------|
| `EmptyHref` | - | `link href cannot be empty` |
| `UnrecognizedFormat` | - | `input is not a recognized link format` |
| `MalformedHtml` | `String` | `malformed HTML link: {0}` |
| `MalformedMarkdown` | `String` | `malformed markdown link: {0}` |
| `MissingHref` | - | `link is missing href/url` |
| `InvalidStyle` | `StylesheetError` (from) | `invalid CSS style: {0}` |
| `InvalidTarget` | `value` | `invalid link target '{value}'` |

### Key variant: `MalformedMarkdown`

Markdown link parsing produces several distinct failure modes (`unmatched '['`, `expected '(' after display text`, `unmatched '('`) but they all share a single variant with an opaque string. The user has no visual pointer to where in their input the problem occurred.

**Block-style proposal:**

```
LinkError: malformed markdown link
┃ unmatched '(' in link
┃
┃ input:
┃   [Click](https://example.com "Title"
┃                            ^^^^^^^^
┃ hint: every '(' must have a matching ')'.
┃   Markdown links use the pattern: [display](url "optional title")
```

**Implementation notes:**

- `Status` title: bold-red `LinkError:` + bold `malformed markdown link`
- `StatusBlock::body()`: use a `Prose`-rendered code block showing the input with a caret pointer. The `Prose` styling can use `{{red}}` for the error indicator and `{{dim}}` for context
- Currently the raw input string is not carried by `MalformedMarkdown` -- only a description string. To enable this pattern, the variant should be extended to carry the original input:

```rust
MalformedMarkdown {
    message: String,
    input: Option<String>,
}
```

This preserves backward compatibility (existing `MalformedMarkdown("reason".into())` call sites still work via `From<&str>` or a conversion) while allowing block-style rendering when the input is available.

### Key variant: `UnrecognizedFormat`

When `Link::try_from` receives input that is neither HTML nor Markdown, the bare message provides no guidance on what was expected.

**Block-style proposal:**

```
LinkError: unrecognized link format
┃ input must start with '<' (HTML) or '[' (Markdown)
┃
┃ examples:
┃   HTML:     <a href="https://example.com">Click</a>
┃   Markdown: [Click](https://example.com)
```

---

## ImageRefError

**File:** `darkmatter/lib/src/render/image_ref.rs:28`

10 variants covering image reference construction, HTML/Markdown parsing, style validation, and attribute value parsing.

| Variant | Fields | Current message |
|---------|--------|-----------------|
| `EmptySource` | - | `image source URL cannot be empty` |
| `MissingSource` | - | `image reference must define either 'src' or 'srcset'` |
| `UnrecognizedFormat` | - | `input is not a recognized image reference format` |
| `MalformedHtml` | `String` | `malformed HTML image reference: {0}` |
| `MalformedMarkdown` | `String` | `malformed markdown image reference: {0}` |
| `InvalidStyle` | `StylesheetError` (from) | `invalid CSS style: {0}` |
| `InvalidDecoding` | `value` | `invalid image decoding value '{value}'` |
| `InvalidFetchPriority` | `value` | `invalid image fetchpriority value '{value}'` |
| `InvalidLoading` | `value` | `invalid image loading value '{value}'` |
| `InvalidReferrerPolicy` | `value` | `invalid image referrerpolicy value '{value}'` |

### Key variant: `InvalidReferrerPolicy`

The `ReferrerPolicy` enum has 8 possible values. A typo like `strict-origin-cross-origin` produces a bare error with no enumeration of valid options.

**Block-style proposal:**

```
ImageRefError: invalid referrerpolicy value
┃ 'strict-origin-cross-origin' is not a valid referrer policy
┃
┃ did you mean: strict-origin-when-cross-origin
┃
┃ valid values:
┃   no-referrer  no-referrer-when-downgrade  origin
┃   origin-when-cross-origin  same-origin
┃   strict-origin  strict-origin-when-cross-origin
┃   unsafe-url
```

**Implementation notes:**

- `Status` title: bold-red `ImageRefError:` + bold `invalid referrerpolicy value`
- `StatusBlock` body: a `Prose`-formatted list of all `ReferrerPolicy` variants, potentially using `strum` or a manual `iter()` for the list
- A fuzzy-match hint (`did you mean`) could be computed using a simple Levenshtein distance against all valid enum values. This is especially useful for the longer hyphenated values

### Key variant: `MalformedMarkdown`

Similar to `LinkError::MalformedMarkdown`, image parsing failures lose the user's input context. The parser detects `unmatched '['`, `expected '(' after alt text`, `unmatched '('`, and `unexpected trailing content` but only stores a description string.

**Block-style proposal:**

```
ImageRefError: malformed markdown image reference
┃ unexpected trailing content after markdown image
┃
┃ input:
┃   ![Cat](cat.png) extra text
┃                   ^^^^^^^^^^^
┃ hint: markdown images use the pattern: ![alt](url "optional title")
┃   Anything after the closing ')' is not valid.
```

**Implementation notes:**

- Same approach as `LinkError::MalformedMarkdown`: extend the variant to carry the original input for block-style rendering
- The `StatusBlock` body renders the input with a `Prose`-styled caret line

---

## Summary

| Error | Key variant | Block-style benefit |
|-------|-------------|-------------------|
| `StylesheetError` | `PropertyValueTypeMismatch` | Structured expected vs received with concrete examples |
| `StylesheetError` | `InvalidColor` | Enumerate accepted color formats with syntax examples |
| `LinkError` | `MalformedMarkdown` | Show original input with caret pointer to parse failure |
| `LinkError` | `UnrecognizedFormat` | Show expected syntax patterns |
| `ImageRefError` | `InvalidReferrerPolicy` | Enumerate all 8 valid values + fuzzy-match hint |
| `ImageRefError` | `MalformedMarkdown` | Show original input with caret pointer to parse failure |

All proposals use `Status` (bold-red error name + bold title) for the heading and `StatusBlock` (red vertical line, `Prose`-formatted body, optional hint) for the contextual block. The `Prose` component handles inline styling tokens (`{{bold}}`, `{{dim}}`, `{{red}}`, `{{rgb R,G,B}}`) for color-coding property names, values, and examples consistent with the existing `Stylesheet::to_terminal_string()` palette.

# Dim Markdown Syntax

## Overview

Darkmatter introduces a new inline formatting primitive for Markdown: the **dim** span, denoted by surrounding text with the `⌄` character (U+2304, *Down Arrowhead*). This syntax produces text rendered at decreased intensity (ANSI SGR code `2`, also known as "faint" or "dim").

The `⌄text⌄` syntax in Markdown is semantically equivalent to `<dim>text</dim>` in [`biscuit-terminal`'s `Prose`](../../biscuit-terminal/docs/components/prose.md) component. Both produce the same terminal-ready output when rendered.

> **Note:** The `⌄` character is not directly accessible on standard keyboard layouts. It was chosen because its visual form (a downward-pointing wedge) semantically aligns with the concept of "dimming" or reducing intensity. Users will typically insert it via OS character pickers, compose sequences, or copy-paste.

## Syntax

```markdown
⌄dimmed text⌄
```

- **Opening delimiter:** `⌄` (U+2304)
- **Closing delimiter:** `⌄` (U+2304)
- **Content:** Any inline text content
- **Delimiter length:** Single character (unlike `==mark==` which uses two)

## Parsing Rules

The `⌄` delimiter follows the same parsing rules as the `_` token for italics in standard Markdown:

1. **Standard Markdown precedence applies.** `⌄` is parsed as an inline formatting delimiter at the same precedence level as `*` and `_` for emphasis.

2. **Inactive inside code spans.** `⌄` delimiters inside inline code spans (backticks) and fenced code blocks are treated as literal characters.

3. **Unclosed markers are literal.** An opening `⌄` without a matching closing `⌄` within the same text scope is treated as a literal `⌄` character.

4. **Standard nesting rules.** Dim spans may be nested within other inline elements (emphasis, strong, links, etc.) and may contain other inline elements, following standard CommonMark inline nesting rules.

5. **No special escape mechanism.** Escaping follows standard Markdown backslash rules: `\⌄` renders as a literal `⌄`.

6. **Single-character delimiters.** Unlike the `==mark==` syntax which uses two-character delimiters, `⌄dim⌄` uses single-character delimiters on each side.

## Rendering Semantics

### Terminal Output

"Dim" maps to ANSI SGR code `2` (faint / decreased intensity). Rendering is capability-gated:

1. Query `biscuit-terminal`'s capability database for `has_dim_support`.
2. **If the terminal supports dim:** emit `\x1b[2m` before the dimmed content and `\x1b[0m` (or the appropriate reset sequence) after.
3. **If the terminal does not support dim:** emit nothing — the text falls back to normal intensity with no visible difference.

This follows the same pattern as `ItalicMode::Auto` and other capability-gated formatting in darkmatter's terminal renderer.

### HTML Output

HTML rendering is **out of scope** for this feature. The dim syntax is terminal-first and does not define an HTML representation. Documents that require HTML output should use standard Markdown emphasis or HTML tags directly.

## Backwards Compatibility

This is a **hard breaking change**:

- `⌄` is redefined as a formatting delimiter immediately upon release.
- Documents containing literal `⌄` characters must be updated to escape them (`\⌄`) or replace them.
- There is **no deprecation period**, **no opt-in mechanism**, and **no feature gate**.

This is acceptable because:
1. Darkmatter is pre-1.0.
2. The `⌄` character is extremely rare in natural-language text and in code.
3. The character is visually distinctive, making accidental usage unlikely.

## Examples

### Positive Examples

#### Basic dim span
```markdown
This is ⌄dimmed⌄ text.
```
Terminal output (when supported): `This is \x1b[2mdimmed\x1b[0m text.`

#### Equivalence with Prose
```rust
use biscuit_terminal::components::prose::Prose;

let prose = Prose::new("The <dim>dim</dim> text should be ignored").render(None);
let md = darkmatter::markdown::Markdown::from("The ⌄dim⌄ text should be ignored");
// Both `prose` and terminal-rendered `md` produce the same output
```

#### Nested in emphasis
```markdown
This is *⌄dimmed and italic⌄* text.
```

#### Inside strong
```markdown
This is **⌄dimmed and bold⌄** text.
```

#### Multiple dim spans
```markdown
⌄first⌄ and ⌄second⌄ dimmed regions.
```

#### Adjacent to punctuation
```markdown
(⌄parenthetical note⌄)
```

### Negative Examples

#### Inside inline code (not processed)
```markdown
Use the `⌄` character for dim spans.
```
The `⌄` inside backticks is rendered literally.

#### Inside fenced code blocks (not processed)
````markdown
```
This ⌄text⌄ is inside a code block.
```
````
Both `⌄` characters are rendered literally.

#### Unclosed marker (rendered literally)
```markdown
This is ⌄unclosed dim text.
```
The single `⌄` is rendered as a literal character: `This is ⌄unclosed dim text.`

#### Empty dim span
```markdown
⌄⌄
```
Produces a zero-length dim span (start and end immediately adjacent). This is syntactically valid but produces no visible content.

## Edge Cases and Limitations

1. **No intra-word dimming by default.** Like standard emphasis with `_`, `⌄` inside a word may be parsed as literal depending on CommonMark's intra-word emphasis rules. Use `*⌄word⌄*` or explicit spacing if intra-word styling is needed.

2. **Unicode width.** The `⌄` character has a standard East Asian Width of neutral and a display width of 1 column in most terminals. It is not a combining character.

3. **Font coverage.** Some older terminals or fonts may not render U+2304. In such cases the character may appear as a replacement glyph (�) or tofu. This does not affect the parsing semantics.

4. **Copy-paste hazards.** Because `⌄` is not keyboard-accessible, users may inadvertently paste lookalike characters (e.g., `˅` U+02C5, `∨` U+2228, `v` U+0076). Only U+2304 is recognized as the dim delimiter.

5. **No HTML fallback.** When rendering to HTML (via `--output html`), `⌄text⌄` will pass through as literal characters unless an HTML renderer is added in a future phase.

## Implementation Notes

- Parsing should be integrated into the existing inline processor pipeline alongside `==mark==` (currently handled by `MarkProcessor`).
- A new `InlineTag::Dim` variant will be added to the `InlineEvent` enum.
- Terminal rendering will track `in_dim` state alongside existing `in_emphasis`, `in_strong`, `in_strikethrough`, and `in_mark` flags.
- Capability detection (`has_dim_support`) will be added to `biscuit-terminal`'s detection module.

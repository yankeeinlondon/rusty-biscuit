# Prose+ Functional Specification

The `Prose` struct provides a way to describe formatting and text in an efficient and intuitive way. The current state is documented in `@biscuit-terminal/docs/components/prose.md`.

In this feature, we extend `Prose`'s ability to detect styles in text and add support for browser-based rendering.

## Goals

- Enhance `Prose` with common Markdown syntax support for links, bold, and italics.
- Maintain compatibility with existing tag-based formatting.
- Implement `BrowserRenderable` to allow `Prose` content to be rendered in web environments with high fidelity to terminal layouts.

## Functional Requirements

### 1. Markdown Parsing

`Prose` will be updated to detect and parse specific Markdown constructs in addition to the existing tag-based formats:

- **Links**: `[desc](reference)` will be treated as a semantic link alias.
- **Bold**: `**text**` will be treated as bold text, equivalent to `<b>text</b>`.
- **Italics**: `_text_` will be treated as italics text, equivalent to `<i>text</i>`.

#### Markdown Flavor Constraints
To maintain simplicity and predictability in the terminal environment, we adhere to a strict subset of Markdown:
- **Bold**: Only `**text**` is supported. `__text__` is NOT supported at this time.
- **Italics**: Only `_text_` is supported. `*text*` is NOT supported at this time.

#### Escaping Mechanism
A backslash `\` escapes the immediately following character, treating it as literal text.
- To render a literal backslash, it must be escaped as `\\`.
- This mechanism applies to all Markdown syntax characters: `*`, `_`, `[`, `]`, `(`, and `)`.
- Escaped characters are ignored by the Markdown processor and passed to the text renderer as literals.

### 2. Terminal Rendering (`Renderable`)

When rendering to a terminal via the `Renderable` trait:

- **Links**:
  - If the terminal supports **OSC8**, the renderer will emit the OSC8 escape sequence for the link.
  - If the terminal does **not** support OSC8, it will fall back to displaying the original Markdown format: `[desc](reference)`.
- **Bold/Italics**: Will use standard ANSI SGR escape sequences as they do today.

### 3. Browser Rendering (`BrowserRenderable`)

We will extend `Prose` to support the `BrowserRenderable` trait, producing a "High-Fidelity Block" for web display.

- **Layout**: The output will be wrapped in a `<div>` that replicates terminal-specific layout (alignment, margins) using inline CSS (e.g., `margin-left: 2ch`, `text-align: center`).
- **Text Styling**: Content will use `<span>` tags for styled text segments.
- **Links**: Rendered as standard `<a>` tags.

## Technical Design

### Parsing Strategy: Pre-processing

To ensure that nesting and layer-tracking logic is inherited from the existing `Prose` implementation, a pre-processing step will be introduced.

#### Token Conversion Order
The "Token Conversion" step must follow a strict execution order to ensure safety and prevent corruption of nested content (e.g., underscores inside URLs):

1. **Links**: `[desc](ref)` -> `<a href="ref">desc</a>`. 
   - This must be performed first to "lock" the link target.
   - The conversion of link targets must be atomic to protect any underscores or asterisks within the URL from subsequent bold/italics processing (e.g., `get_user_profile` in a URL should not have its underscores interpreted as italics).
2. **Bold**: `**text**` -> `<b>text</b>`.
3. **Italics**: `_text_` -> `<i>text</i>`.

#### Recursive Parsing
After token conversion, the existing `Prose` parsing engine processes the resulting string, ensuring consistent behavior for nested styles and attributes.

## Acceptance Criteria

- [ ] `Prose` correctly parses `**bold**`, `_italics_`, and `[desc](ref)` syntax.
- [ ] `Prose` strictly ignores `__bold__` and `*italics*`.
- [ ] `Prose` correctly handles the escaping mechanism:
  - `\\` renders as `\`.
  - `\*`, `\_`, `\[`, etc., render as literal characters.
- [ ] Pre-processing follows the mandated order (Links -> Bold -> Italics).
- [ ] Link target conversion is atomic and protects internal Markdown-like characters (e.g., underscores in URLs).
- [ ] Terminal rendering emits OSC8 for links on supported terminals.
- [ ] Terminal rendering falls back to `[desc](ref)` for links on non-OSC8 terminals.
- [ ] `BrowserRenderable` implementation produces a `<div>` "High-Fidelity Block" with inline CSS for margins and alignment.
- [ ] `BrowserRenderable` uses `<span>` tags for styled segments within the output block.
- [ ] Nested styles (e.g., `**_bold italics_**`) are correctly preserved across all rendering targets.

## Technical Constraints

- Must remain fully backward compatible with the existing `<a href="...">`, `<b>`, and `<i>` tag formats.
- Pre-processing must be performed carefully to handle escaped characters and avoid collisions with existing tags.
- `BrowserRenderable` output must prioritize visual parity with terminal layout constraints.

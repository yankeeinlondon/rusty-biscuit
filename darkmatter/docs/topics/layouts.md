# Layouts and Renderable Components

Every terminal component in `biscuit-terminal` owns a `Layout` that controls how its content is positioned: margins, alignment, word-wrapping, row fill, and background color. The `Layout` struct lives in `biscuit_terminal::utils::layout` and is the single mechanism through which all renderable components negotiate their space on screen.

This document walks through the Layout system from simple to complex, showing how each piece works and how they compose together.

## The Layout Struct

```rust
pub struct Layout {
    pub left_margin: Margin,
    pub right_margin: Margin,
    pub top_margin: Margin,
    pub bottom_margin: Margin,
    pub alignment: Alignment,
    pub row_fill_strategy: RowFill,
    pub word_wrap: WordWrap,
    pub page_bg_color: Option<Color>,
}
```

The default layout has no margins, left alignment, no word-wrap, auto row-fill, and no background color. Every component that implements `Renderable` stores one of these and exposes it via `layout()` / `layout_mut()`.

## Basic Examples

### Margins

Margins add whitespace around the rendered content. The simplest form is fixed character margins:

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::utils::layout::Margin;

let prose = Prose::new("Hello, world!")
    .left_margin(Margin::Chars(4))
    .right_margin(Margin::Chars(4));

let term = Terminal::default();
print!("{}", prose.display(&term));
// Output (on 80-col terminal):
//     Hello, world!
//  ^^^^ left margin
```

Percentage margins scale with the terminal width:

```rust
let prose = Prose::new("Centered-ish content")
    .left_margin(Margin::Percent(10.0))
    .right_margin(Margin::Percent(10.0));
// At 100 columns: 10 char margins on each side → 80 cols available
// At 120 columns: 12 char margins on each side → 96 cols available
```

### Alignment

Alignment positions content horizontally within the space between margins:

```rust
use biscuit_terminal::utils::layout::Alignment;

// Right-aligned in the available width
let prose = Prose::new("Status: OK")
    .alignment(Alignment::Right);

// Centered with margins — alignment applies inside the margin boundaries
let prose = Prose::new("Title Text")
    .left_margin(Margin::Chars(10))
    .right_margin(Margin::Chars(10))
    .alignment(Alignment::Center);
```

On an 80-column terminal with 10-char margins on each side, `Alignment::Center` centers the text within the remaining 60 columns.

### Word Wrapping

The `WordWrap` enum controls what happens when content exceeds the available width:

```rust
use biscuit_terminal::utils::layout::WordWrap;

// Word-wrap with hyphenation on overflow (default start-looking window: 8 chars)
let prose = Prose::new("A very long paragraph that needs to be wrapped...")
    .word_wrap(WordWrap::WrapProse(Some(8), None));

// Word-wrap with hanging indent — continuation lines indented 4 spaces
let prose = Prose::new("• First bullet point with a long description that wraps")
    .word_wrap(WordWrap::WrapProse(Some(8), Some(4)));

// Truncate with ellipsis instead of wrapping
let prose = Prose::new("This line will be cut off if it's too long")
    .word_wrap(WordWrap::Truncate(Some("…".to_string())));

// No wrapping — hard break at the boundary, no indicators
let prose = Prose::new("Code-like content")
    .word_wrap(WordWrap::None);
```

`WrapProse(start_looking, hanging_indent)` begins scanning for a break character (whitespace, hyphen) `start_looking` characters before the line limit. If no break point is found, it hyphenates.

## How Layout Applies to Content

Every `Renderable` implementation follows the same pattern:

1. Get the terminal width from the `Terminal` struct
2. Use `layout.available_width(term_width)` to compute the content area
3. Render the component's content into that constrained width
4. Call `layout.apply_layout(&content, term_width)` to wrap, align, and pad

```rust
impl Renderable for FileTree {
    fn render(&self, term: &Terminal) -> String {
        match &self.model {
            Some(model) => {
                let raw = render::render_model(model, term, self.show_root);
                self.layout.apply_layout(&raw, term.width())
            }
            None => String::new(),
        }
    }
}
```

The `apply_layout` method does the heavy lifting:

1. Resolves margins from `Margin` variants to character counts
2. Splits content into lines
3. Applies word-wrapping per the `word_wrap` policy
4. Adds left-margin padding and alignment spacing to each line
5. Optionally fills rows to the available width (for background colors)
6. Preserves trailing newline semantics from the original content

## The Margin Enum

`Margin` has four variants, each serving a different use case:

```rust
pub enum Margin {
    None,                      // Zero whitespace (default)
    Chars(u32),                // Fixed character count
    Percent(f32),              // Percentage of terminal width
    Offset(Box<Margin>, u32), // Base margin + additional chars (deferred)
}
```

The `Offset` variant exists for nesting. When a child component inherits a parent's percentage margin and adds its own character offset, the percentage can't be resolved yet — the terminal width isn't known until render time. `Offset` defers that resolution:

```rust
// Parent has a 10% margin
let parent_margin = Margin::Percent(10.0);

// Child adds 2 more characters on top
let child_margin = parent_margin.add_chars(2);
// Result: Margin::Offset(Box::new(Margin::Percent(10.0)), 2)

// At render time on a 100-col terminal:
// resolve(Percent(10.0), 100) + 2 = 12 characters
```

The `add_chars` method optimizes common cases: `None + n` becomes `Chars(n)`, `Chars(a) + b` becomes `Chars(a + b)`, and everything else wraps in `Offset`.

## Row Fill and Background Colors

`RowFill` controls whether lines are padded with spaces to fill the available width:

```rust
pub enum RowFill {
    Auto,   // Fill only when page_bg_color is set (default)
    Fill,   // Always pad to full width
    Exact,  // Never pad — natural content width only
}
```

This matters when you set a background color. Without row fill, the background only extends to the end of each line's visible content, creating a ragged right edge:

```rust
use biscuit_terminal::utils::layout::{Layout, RowFill};
use biscuit_terminal::utils::color::Color;

// Background without fill — ragged edge
let layout = Layout {
    page_bg_color: Some(Color::Ansi256(236)),
    row_fill_strategy: RowFill::Exact,
    ..Layout::default()
};
// "Hello     " ← bg ends here
// "World" ← bg ends here (shorter)

// Background with fill (Auto handles this automatically)
let layout = Layout {
    page_bg_color: Some(Color::Ansi256(236)),
    row_fill_strategy: RowFill::Auto,  // default
    ..Layout::default()
};
// "Hello          " ← bg fills to available width
// "World          " ← bg fills to same width
```

## Fluent Builder API

The `Renderable` trait provides builder methods that return `Self`, so you can chain layout configuration on any component:

```rust
let section = Section::new(HeadingLevel::h2, "Getting Started")
    .left_margin(Margin::Chars(2))
    .right_margin(Margin::Chars(2))
    .alignment(Alignment::Center)
    .word_wrap(WordWrap::WrapProse(Some(8), Some(4)))
    .row_fill_strategy(RowFill::Fill);
```

Or replace the entire layout at once:

```rust
let custom_layout = Layout {
    left_margin: Margin::Percent(15.0),
    right_margin: Margin::Percent(15.0),
    alignment: Alignment::Center,
    word_wrap: WordWrap::WrapProse(Some(8), None),
    ..Layout::default()
};

let prose = Prose::new("Narrow centered content")
    .with_layout(custom_layout);
```

## Rendering Methods

Components expose three rendering paths, each with different layout behavior:

| Method               | Terminal-aware | Trailing `\n` | Use for                    |
|----------------------|----------------|---------------|----------------------------|
| `render(term)`       | Yes            | No            | Composition, embedding     |
| `render_optimistic(width)` | No       | No            | Composition, no Terminal   |
| `display(term)`      | Yes            | Yes           | Direct terminal output     |

- **`render()`** uses `Terminal` for width, color depth, font detection, etc. Output is meant for embedding inside other components — no trailing newline.
- **`render_optimistic()`** assumes modern capabilities and optionally takes a width (defaults to 80). Same composition semantics.
- **`display()`** wraps `render()` and guarantees a trailing newline. This is what CLI programs should use when printing directly to the terminal.

```rust
// Composing: embed one component's output inside another
let inner = Prose::new("nested content").render(&term);
let outer = format!("Box: [{}]", inner);

// Direct output: display() ensures clean terminal behavior
print!("{}", table.display(&term));
```

## Composition Patterns

### Vertical Stacking with Compose

`Compose` stacks multiple components vertically, then applies its own layout to the combined output:

```rust
let mut doc = Compose::default();
doc.add_heading("Project Overview", 1)
   .add_text("This project contains ")
   .add_prose(Prose::new("<b>important</b> files"))
   .add_text(" for processing.\n");

// The Compose's layout applies to the entire stacked output
let doc = doc
    .left_margin(Margin::Chars(2))
    .alignment(Alignment::Center);

print!("{}", doc.display(&term));
```

Each part renders independently, then `Compose` concatenates the results and runs `apply_layout` over the combined string.

### Side-by-Side with TwoColumn

`TwoColumn` places two content blocks horizontally, splitting available width between them:

```rust
use biscuit_terminal::components::two_column::{TwoColumn, ColumnWidth};

let cols = TwoColumn::new(
    Prose::new("Left column content here"),
    Prose::new("Right column content"),
)
.with_left_width(ColumnWidth::Percent(0.6))  // 60/40 split
.with_gap(3);                                 // 3-char gap between columns
```

Column width resolution:
1. Fixed widths (`ColumnWidth::Fixed(30)`) are used as-is
2. Percentages (`ColumnWidth::Percent(0.6)`) multiply against the available width after subtracting the gap
3. If either column would be less than 1 character, content stacks vertically as a fallback

### Nesting with Parent Layouts

When a component is rendered inside another, it can inherit the parent's margins and add its own offset:

```rust
let parent_layout = Layout {
    left_margin: Margin::Percent(10.0),
    right_margin: Margin::Percent(10.0),
    ..Layout::default()
};

// Child inherits parent margins and adds 4 chars of indent
let child = Prose::new("Nested content")
    .with_parent_layout(&parent_layout, 4, 0);
// child.left_margin = Offset(Percent(10.0), 4)
// child.right_margin = Percent(10.0)
```

This pattern is used by `Table`, `Section`, and other container components when they render children. The key insight is that `with_parent_layout` uses `Margin::add_chars` internally, which defers percentage resolution via `Offset`. This means a parent's `Percent(10.0)` margin isn't locked in at 8 characters just because someone measured it at 80 columns — it re-resolves whenever the child renders.

## Advanced: Custom Renderable Implementation

To build a custom component with layout support, implement `Renderable` and store a `Layout`:

```rust
use biscuit_terminal::prelude::*;
use biscuit_terminal::utils::layout::Layout;
use std::any::Any;

#[derive(Debug)]
struct StatusBar {
    label: String,
    value: String,
    layout: Layout,
}

impl StatusBar {
    fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            layout: Layout::default(),
        }
    }
}

impl Renderable for StatusBar {
    fn render(&self, term: &Terminal) -> String {
        // Step 1: Calculate available width from layout
        let available = self.layout.available_width(term.width());

        // Step 2: Render content into that constraint
        let content = format!("{}: {}", self.label, self.value);
        let truncated = if content.len() > available as usize {
            format!("{}…", &content[..available as usize - 1])
        } else {
            content
        };

        // Step 3: Apply layout (margins, alignment, wrapping)
        self.layout.apply_layout(&truncated, term.width())
    }

    fn layout(&self) -> &Layout { &self.layout }
    fn layout_mut(&mut self) -> &mut Layout { &mut self.layout }
    fn as_any(&self) -> &dyn Any { self }
}

// Usage:
let bar = StatusBar::new("Build", "passing")
    .left_margin(Margin::Chars(2))
    .alignment(Alignment::Right);
print!("{}", bar.display(&term));
```

The three-step pattern — `available_width` → render content → `apply_layout` — is the standard way every component interacts with its layout. The layout handles all the margin resolution, wrapping, and alignment; the component just needs to render its content within the given width constraint.

## Summary

| Concept | Type | Purpose |
|---------|------|---------|
| `Margin` | enum | Whitespace around content (fixed, percent, or composed) |
| `Alignment` | enum | Horizontal positioning (left, center, right) |
| `WordWrap` | enum | Overflow handling (wrap, truncate, none) |
| `RowFill` | enum | Line padding strategy (auto, fill, exact) |
| `Layout` | struct | Bundles all of the above plus background color |
| `Renderable` | trait | Components own a Layout, expose builder methods |
| `Compose` | struct | Vertical stacking of components |
| `TwoColumn` | struct | Horizontal side-by-side layout |
| `with_parent_layout` | method | Nested margin inheritance with deferred resolution |

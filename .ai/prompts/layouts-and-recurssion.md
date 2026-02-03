# Design Document: Layouts and Recursion in biscuit-terminal

## Problem Statement

A terminal may have certain features it can or can not handle (e.g., OSC8 links, etc.) but the constraint we're interested in solving for now is non-specific to which standards the caller's terminal app supports but one of the key fundamental constraints of any terminal window ... its width! Whereas a terminal has an _unlimited_ height because terminals scroll, it only has a set number of characters in width which it can render to.

The **biscuit-terminal** package defines a `Layout` struct whose intent is to allow _renderable_ components to express their layout constraints ahead of time (aka, before knowing how many columns will ACTUALLY be present when it is time to render).

> The `Layout` struct exists to describe **how to constrain** a future render window, not to store computed values.

The `Renderable` trait requires any _renderable_ component to provide two render methods; both of which currently take an optional `Layout` as a parameter. This design works reasonably well when the `Terminal` can report on the ACTUAL width available at render time, but only works well when rendering a singular or flat list of _renderable_ components. At the point where recursion becomes a requirement, however, this solution starts to be unworkable because the `left_margin` and `right_margin` which constrain the "available width" must be compounded at each nesting level.

### Example

- An `OrderedList` _contains_ a vector of list items
- One of those list items _could be_ another `OrderedList` (with its own list items underneath)
- Indeed the child `OrderedList` may too have a child `OrderedList`, etc.
- When we render the component, let's imagine we start with:
    - a width of 80 characters in the terminal, and
    - a left and right margin of 0
- A characteristic of `OrderedList` is that nested lists are indented by some amount
    - for this reason we add an `indent_children` property to the `OrderedList` as the PARENT; let's assume it defaults to 4
- The parent list renders and correctly starts with no left indent. It iterates over its items, and when it hits another list it knows this is a "child" and therefore must ensure that the `indent_children` rule is observed by the child list:
    - When a parent list sees a child element it DOES NOT render its bullet point; it simply hands off to the child
    - But the parent list will adjust the `left_margin` property of the Layout to be: `left_margin` (currently 0) + `indent_children` (defaulted to 4)
    - Now the render method of the child list is called, but because its `left_margin` has been set, it will automatically indent itself correctly

---

## Core Design Principle: Lazy Constraints

**Key invariant**: Layout properties should be assignable without knowledge of the render window size.

All Layout properties are _lazy_ - they describe constraints that get evaluated at render time. The `Margin` enum is a perfect example:

```rust
pub enum Margin {
    None,
    Chars(u32),      // Fixed constraint
    Percent(f32),    // Relative constraint - needs terminal width to evaluate
}
```

This means we must NOT add computed/render-time values as Layout properties:

- `available_width: u32` (computed value)
- `nesting_depth: u32` (render-time state)
- `parent_prefix_width: u32` (component-specific state)

---

## Proposed Solution

### Key Architectural Change: Components Own Their Layout

Currently, `Layout` is passed externally via the `Renderable` trait's render methods. This makes recursive constraint propagation awkward because each render call must manually thread layout through.

**New approach**: Every component owns a `Layout` field. The `default()` and `new()` constructors initialize it to `Layout::default()`. Callers can modify it via builder methods before rendering.

```rust
let normal_list = OrderedList::new(vec!["one", "two", "three"]);
let indented_list = OrderedList::new(vec!["one", "two", "three"])
    .left_margin(Margin::Chars(4));
```

### Revised Renderable Trait

```rust
pub trait Renderable: std::fmt::Debug {
    /// Render without access to an actual terminal.
    /// Pass a terminal width if known; defaults to 80 if `None`.
    fn render(&self, term_width: Option<u32>) -> String;

    /// Render with access to the actual terminal (width determined by terminal).
    fn fallback_render(&self, term: &Terminal) -> String;

    /// Returns `true` if this component renders as a block-level element
    /// (lists, block quotes, tables, etc.) that manages its own line structure.
    ///
    /// Parent containers use this to decide whether to prepend decorations
    /// (bullets, numbers) or simply hand off rendering to the child.
    fn is_block_level(&self) -> bool {
        false
    }

    // --- Layout builder methods ---
    // These allow uniform layout configuration across all components.
    // They require `Self: Sized` and are not available through trait objects.

    fn left_margin(self, margin: Margin) -> Self where Self: Sized;
    fn right_margin(self, margin: Margin) -> Self where Self: Sized;
    fn top_margin(self, margin: Margin) -> Self where Self: Sized;
    fn bottom_margin(self, margin: Margin) -> Self where Self: Sized;
    fn alignment(self, alignment: Alignment) -> Self where Self: Sized;
    fn row_fill_strategy(self, strategy: RowFill) -> Self where Self: Sized;
    fn word_wrap(self, wrap: WordWrap) -> Self where Self: Sized;

    /// Configure this component as a child of the given parent layout.
    /// Offsets the left and right margins by the specified amounts.
    fn as_child_of(self, parent: &Layout, left_offset: u32, right_offset: u32) -> Self
    where
        Self: Sized;
}
```

**Object safety**: The builder methods require `Self: Sized`, so they are NOT callable through `dyn Renderable`. This is correct because:

- Layout configuration happens at construction time on concrete types
- `render()` and `fallback_render()` remain object-safe for `Arc<dyn Renderable>`

### Reducing Boilerplate: Layout Access

Since every component owns a `Layout`, the builder methods all do the same thing: modify `self.layout.{field}`. To avoid implementing these identically in every component, we add a required accessor:

```rust
pub trait Renderable: std::fmt::Debug {
    // ... render methods ...

    /// Access the component's layout for reading.
    fn layout(&self) -> &Layout;

    /// Access the component's layout for mutation.
    fn layout_mut(&mut self) -> &mut Layout;

    // Builder methods get default implementations:
    fn left_margin(mut self, margin: Margin) -> Self where Self: Sized {
        self.layout_mut().left_margin = margin;
        self
    }
    fn right_margin(mut self, margin: Margin) -> Self where Self: Sized {
        self.layout_mut().right_margin = margin;
        self
    }
    // ... etc. for all layout properties ...

    fn as_child_of(mut self, parent: &Layout, left_offset: u32, right_offset: u32) -> Self
    where
        Self: Sized,
    {
        let layout = self.layout_mut();
        layout.left_margin = parent.left_margin.add_chars(left_offset);
        layout.right_margin = parent.right_margin.add_chars(right_offset);
        self
    }
}
```

Implementors only need to provide `layout()` and `layout_mut()`:

```rust
impl Renderable for OrderedList {
    fn layout(&self) -> &Layout { &self.layout }
    fn layout_mut(&mut self) -> &mut Layout { &mut self.layout }

    fn render(&self, term_width: Option<u32>) -> String { /* ... */ }
    fn fallback_render(&self, term: &Terminal) -> String { /* ... */ }
    fn is_block_level(&self) -> bool { true }
}
```

### Margin Composition: The `add_chars` Method

When a parent offsets a child's margin, we need to add a fixed character amount to a potentially lazy margin. This requires a new Margin method and variant:

```rust
pub enum Margin {
    None,
    Chars(u32),
    Percent(f32),
    /// A base margin plus a fixed character offset.
    /// Resolved at render time: resolve(base) + offset.
    Offset(Box<Margin>, u32),
}

impl Margin {
    /// Add a fixed character offset to this margin.
    /// Preserves lazy evaluation of the base margin.
    pub fn add_chars(self, chars: u32) -> Margin {
        if chars == 0 {
            return self;
        }
        match self {
            Margin::None => Margin::Chars(chars),
            Margin::Chars(existing) => Margin::Chars(existing + chars),
            other => Margin::Offset(Box::new(other), chars),
        }
    }
}
```

And `resolve_margin` handles the new variant:

```rust
fn resolve_margin(margin: &Margin, terminal_width: u32) -> u32 {
    match margin {
        Margin::None => 0,
        Margin::Chars(chars) => *chars,
        Margin::Percent(pct) => ((terminal_width as f32) * pct / 100.0).round() as u32,
        Margin::Offset(base, extra) => Self::resolve_margin(base, terminal_width) + extra,
    }
}
```

This is key: a parent with `Margin::Percent(10.0)` can still have a child offset by 4 chars, and both remain lazy until render time.

---

## How Recursive Rendering Works

### Step-by-Step: Nested OrderedList

```
Terminal width: 80
Parent OrderedList: layout.left_margin = 0, indent_children = 4
  ├─ "First item"    (String)
  ├─ "Second item"   (String)
  └─ OrderedList     (Component, is_block_level = true)
       ├─ "Nested A" (String)
       └─ "Nested B" (String)
```

**Parent's render method** (term_width = 80):

1. Evaluates own layout: `left_margin = 0, right_margin = 0 → available = 80`
2. Item "First item" → String → prepend `"1. "` → output `"1. First item"`
3. Item "Second item" → String → prepend `"2. "` → output `"2. Second item"`
4. Item OrderedList → Component → `is_block_level() = true`
   - **No bullet/number prefix** for block-level children
   - Construct a child layout by offsetting margins: `child_left_margin = 0 + 4 = Chars(4)`
   - The parent has two options for how to apply this:
     - **Option A** _(preferred)_: Reduce the `term_width` passed to the child by 4, then indent the child's output by 4 spaces
     - **Option B**: Modify the child's layout left_margin before rendering (requires mutability or clone)

**Using Option A** (reduce width + indent output):

1. Call `child.render(Some(76))` — child gets 76 chars of width
2. Child renders normally within 76 chars:
   - `"1. Nested A"`
   - `"2. Nested B"`
3. Parent indents each line by 4 spaces:
   - `"    1. Nested A"`
   - `"    2. Nested B"`

**Final output:**

```
1. First item
2. Second item
    1. Nested A
    2. Nested B
```

### Why This Works

Each nesting level:

1. The parent reduces the width available to the child
2. The child evaluates its own layout constraints against the reduced width
3. The parent adds visual indentation to the child's output

The child **does not need to know it's nested**. It just renders into whatever width it's given. The parent handles the indentation and width reduction.

Width compounds naturally:

```
Level 0: term_width = 80, indent = 0,  child gets 76 (80 - 4)
Level 1: term_width = 76, indent = 4,  child gets 72 (76 - 4)
Level 2: term_width = 72, indent = 8,  child gets 68 (72 - 4)
```

---

## `is_block_level()`: Block vs Inline Content

The parent needs to know whether a `RenderableContent::Component` should receive a bullet/number decoration or be handed off as a block:

| Component | `is_block_level()` | Parent behavior |
|-|-|-|
| `OrderedList` | `true` | No bullet, indent + reduce width |
| `UnorderedList` | `true` | No bullet, indent + reduce width |
| `BlockQuote` | `true` | No bullet, indent + reduce width |
| `Table` | `true` | No bullet, indent + reduce width |
| `Section` | `true` | No bullet, indent + reduce width |
| `Prose` | `false` | Add bullet + render inline |
| `TextBlock` | `false` | Add bullet + render inline |

---

## Changes to List Components

### OrderedList

```rust
pub struct OrderedList {
    items: Vec<RenderableContent>,
    layout: Layout,

    /// How many characters to indent child block-level items.
    /// Defaults to 4.
    indent_children: u32,

    /// Starting number (default: 1).
    start: u32,
}
```

### UnorderedList

```rust
pub struct UnorderedList {
    items: Vec<RenderableContent>,
    layout: Layout,
    bullet: String,
    hanging_indent: bool,

    /// How many characters to indent child block-level items.
    /// Defaults to bullet width.
    indent_children: Option<u32>,
}
```

---

## Layout Available Width

Add a method to Layout that evaluates the lazy constraints against a given terminal width:

```rust
impl Layout {
    /// Calculate the available content width by evaluating margin constraints
    /// against the provided terminal width.
    pub fn available_width(&self, terminal_width: u32) -> u32 {
        let left = Self::resolve_margin(&self.left_margin, terminal_width);
        let right = Self::resolve_margin(&self.right_margin, terminal_width);
        terminal_width.saturating_sub(left).saturating_sub(right)
    }
}
```

This method is purely functional — it evaluates lazy constraints on demand without storing the result.

---

## Implementation Plan

### Stage 1: Margin::Offset Variant

1. Add `Offset(Box<Margin>, u32)` to Margin enum
2. Update `resolve_margin` to handle it
3. Add `Margin::add_chars(self, u32) -> Margin` method
4. Add tests for margin composition

### Stage 2: Layout Additions

1. Add `available_width(&self, u32) -> u32` method to Layout
2. Add tests

### Stage 3: Renderable Trait Redesign

1. Change `render(&self, layout: Option<&Layout>)` → `render(&self, term_width: Option<u32>)`
2. Change `fallback_render(&self, term: &Terminal, layout: Option<&Layout>)` → `fallback_render(&self, term: &Terminal)`
3. Add `layout(&self) -> &Layout` and `layout_mut(&mut self) -> &mut Layout`
4. Add builder methods with default implementations
5. Add `is_block_level()` with default `false`
6. Add `as_child_of()` with default implementation

### Stage 4: Add Layout Field to All Components

For each component:

1. Add `layout: Layout` field
2. Initialize in `Default`, `new()`, `From` impls
3. Implement `layout()` and `layout_mut()`
4. Update `render()` and `fallback_render()` to use internal layout
5. Implement `is_block_level()` where appropriate

### Stage 5: Update List Rendering

1. Add `indent_children` field to `OrderedList` and `UnorderedList`
2. Update `render_content()`:
   - String items: add bullet/number prefix
   - Block-level components: reduce width, render, indent output
   - Inline components: add bullet/number prefix + render
3. Add comprehensive tests for nested scenarios

### Stage 6: Update Other Components

1. `BlockQuote`: use layout for border indentation
2. `Section`: pass constrained width to children
3. `Table`: handle block content in cells

---

## Breaking Change Assessment

### Renderable Trait: Breaking

The trait signature changes:

```rust
// Before
fn render(&self, layout: Option<&Layout>) -> String;
fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String;

// After
fn render(&self, term_width: Option<u32>) -> String;
fn fallback_render(&self, term: &Terminal) -> String;
fn layout(&self) -> &Layout;
fn layout_mut(&mut self) -> &mut Layout;
```

All `Renderable` implementors must be updated. This is a **breaking change** but is contained within the biscuit-terminal crate. External consumers of the trait (e.g., darkmatter) will need updates.

### Layout Struct: Additive

- New `Margin::Offset` variant: non-breaking (match arms need update only in resolve_margin)
- New `available_width()` method: additive

### RenderableContent: No Change

```rust
pub enum RenderableContent {
    String(String),
    Component(Arc<dyn Renderable>),
}
```

The enum is unchanged. `Arc<dyn Renderable>` still works because `render()` and `fallback_render()` remain object-safe.

---

## Test Cases

### Margin Composition

```rust
#[test]
fn test_margin_add_chars_to_none() {
    assert_eq!(Margin::None.add_chars(4), Margin::Chars(4));
}

#[test]
fn test_margin_add_chars_to_chars() {
    assert_eq!(Margin::Chars(2).add_chars(4), Margin::Chars(6));
}

#[test]
fn test_margin_add_chars_to_percent() {
    let result = Margin::Percent(10.0).add_chars(4);
    // Should be Offset variant, resolved at render time
    assert_eq!(Layout::resolve_margin(&result, 100), 14); // 10% of 100 + 4
}
```

### Available Width

```rust
#[test]
fn test_available_width() {
    let layout = Layout {
        left_margin: Margin::Chars(10),
        right_margin: Margin::Chars(10),
        ..Layout::default()
    };
    assert_eq!(layout.available_width(80), 60);
}
```

### Nested List: No Double Bullets

```rust
#[test]
fn test_nested_ordered_list() {
    let inner = OrderedList::new(vec!["Nested A", "Nested B"]);
    let outer = OrderedList::from(vec![
        RenderableContent::String("First".into()),
        RenderableContent::Component(Arc::new(inner)),
    ]);

    let result = outer.render(Some(80));

    assert_eq!(result, "1. First\n    1. Nested A\n    2. Nested B");
}
```

### Builder Pattern

```rust
#[test]
fn test_component_builder_layout() {
    let list = OrderedList::new(vec!["one", "two"])
        .left_margin(Margin::Chars(4));

    let result = list.render(Some(80));
    // Should be indented by 4
    assert!(result.starts_with("    1. one"));
}
```

### Width Constraint Propagation

```rust
#[test]
fn test_nested_list_width_constraint() {
    let inner = UnorderedList::new(vec!["A long item that should wrap within constraints"]);
    let outer = UnorderedList::from(vec![
        RenderableContent::Component(Arc::new(inner)),
    ]);

    let result = outer.render(Some(30));

    for line in result.lines() {
        assert!(
            visible_width(line) <= 30,
            "Line exceeds width: '{}'", line
        );
    }
}
```

---

## Open Questions

1. **Should `as_child_of` be on the trait or just a Layout method?**
   - On the trait: convenient builder pattern, works with any component
   - On Layout: simpler trait, but requires `component.layout_mut()` access

2. **Should `RenderableWrapper` be removed or kept alongside?**
   - The current `RenderableWrapper` trait on Layout serves a similar purpose to the new internal layout application
   - May become redundant once components apply their own layout internally

3. **How do components that don't need Layout handle the new required methods?**
   - Option: provide a static `Layout::default()` reference for simple components
   - Option: make `layout()` / `layout_mut()` optional with a blanket fallback

4. **Should `indent_children` be a Layout property or component-specific?**
   - Current proposal: component-specific (only lists need it)
   - Alternative: Layout property for general nesting indent

---

## Conclusion

This design shifts Layout from an externally-passed parameter to an internally-owned property of every component. The key architectural changes:

1. **Components own their Layout** — configured at construction via builder methods
2. **`render(term_width)` replaces `render(layout)`** — terminal width is the render-time input; layout is internal state
3. **Parents constrain children** by reducing the `term_width` passed to child renders and indenting the output
4. **`Margin::Offset`** preserves lazy evaluation when composing parent + child margins
5. **`is_block_level()`** lets parent containers distinguish block vs inline children

The child never needs to know it is nested. It renders into whatever width it is given, using its own layout constraints. The parent handles the visual indentation and width reduction.

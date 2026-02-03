# Design Document: Layouts and Recursion in biscuit-terminal

## Problem Statement

A terminal may have certain features it can or can not handle (e.g., OSC8 links, etc.) but the constraint we're interested in solving for now is non-specific to which standards the caller's terminal app supports but one of the key fundamental constraints of any terminal window ... it's width! Whereas a terminal has an _unlimited_ height because terminals scroll but they only have a set number of characters in width which they can render to.

The **Terminal Biscuit** package defines a `Layout` struct who's intent it is to allow _renderable_ components to be able to express their layout constraints ahead of time (aka, before knowing how many columns will ACTUALLY be present when it is time to render).

> The `Layout` struct exists to describe **how to constrain** a future render window, not to store computed values.

The `Renderable` trait requires any _renderable_ component to provide two render methods; both of which will take an optional `Layout` as a parameter. This design works reasonably well when the `Terminal` can report on the ACTUAL width we have available at render time but only works well when rendering is done on a singular or list of flat _renderable_ components. At the point where recursion starts to become a requirement, however, this solution starts to be unworkable because the `left_margin` and `right_margin` which constrain the "available width" must be compounded at each nesting level.

### Example

- an `OrderedList` _contains_ a vector of list items that make up it's list
- one of those list items _could be_ another `OrderedList` item (with it's own list items underneath)
- indeed the child `OrderedList` item may too have a child `OrderedList`, etc.
- when we render the component, let's imagine we start with:
    - a width of 80 characters in the terminal, and
    - a left and right margin of 0
- a characteristic of `OrderedList`'s is that any nested lists are indented by some amount
    - for this reason we add a `indent_children` property to the `OrderedList` as the PARENT; let's assume it defaults to 4.
- the parent list renders and correctly starts with no left indent, it iterates over it's items and when it hits another list it knows this is a "child" and therefore must ensure that the `indent_children` rule is observed by the child list.
    - when a parent list see's a child element it DOES NOT render it's bullet point, it simply hands off to the child
    - but the parent list will adjust the `left_margin` property of the Layout to be:  `left_margin` (currently 0) + `indent_children` which was defaulted to 4
    - now the render method of the child list is called but because it's left_margin has been set it will automatically indent itself correctly

---

OK so that was my thought process and what I think the solution should look like is:

1. render methods of the `Renderable` trait should be changed to:

    ```rust
    /// **Opportunistic Render**
    ///
    /// renders without knowledge of the underlying terminal's
    /// capabilities with an "opportunistic" view that the
    /// terminal supports all capabilities.
    fn render(&self, layout: &Layout) -> String;

    /// **Fallback Render**
    ///
    /// Renders the component based on the capabilities of the
    /// passed in `Terminal`. Will provide graceful fallbacks
    /// when possible.
    fn fallback_render(&self, layout: &Layout, term: &Terminal) -> String;
    ```

    - added new `term_width` property to both
    - `layout` is no longer be optional because it would become non-ergonomic
    - `layout` and `term` swapped in order just because I feel it is more intuitive

3. we need to add an optional `

## Core Design Principle: Lazy Constraints

The `Layout` struct exists to describe **how to constrain** a future render window, not to store computed values. Properties like margins and alignment are *lazy* - they describe constraints that get evaluated at render time when the actual terminal width is known.

**Key invariant**: Layout properties should be assignable without knowledge of the render window size.

This means we must NOT add properties like:

- `available_width: u32` (computed value)
- `nesting_depth: u32` (render-time state)
- `parent_prefix_width: u32` (component-specific state)

These violate the lazy constraint principle and would make the system brittle.

---

## Current Architecture Analysis

### The Renderable Trait

```rust
pub trait Renderable: std::fmt::Debug {
    fn render(&self, layout: Option<&Layout>) -> String;
    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String;
}
```

The trait passes `Layout` to children, which is correct. The issue is:

- No way for Layout to know the terminal width at render time
- Components don't distinguish block-level vs inline content

### Layout Struct Capabilities

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

All existing properties are **lazy constraints** - they describe *how* to constrain, not *what* the final values are. The `Margin` enum is a perfect example:

```rust
pub enum Margin {
    None,
    Chars(u32),      // Fixed constraint
    Percent(f32),    // Relative constraint - needs terminal width to evaluate
}
```

### Current List Rendering Problem

```rust
// In UnorderedList::render_content()
for item in self.items.iter() {
    result.push_str(&self.bullet);  // ALWAYS adds bullet - even for nested lists!
    let content = match item {
        RenderableContent::Component(c) => c.render(layout),  // Child may have its own bullets
        // ...
    };
}
```

The parent list blindly prepends a bullet to ALL items, including Component items that may be lists with their own bullets.

---

## Design Goals

1. **Preserve lazy constraints**: Layout remains declarative, evaluated at render time
2. **Prevent double decoration**: Parent doesn't add bullet to block-level children
3. **Progressive width constraint**: Each nesting level reduces available width via margins
4. **Backward compatible**: Existing code works without modification
5. **Component-specific state stays in components**: Nesting depth, prefix width, etc. are NOT Layout concerns

---

## Proposed Solution

### Part 1: Add Terminal Width to Layout

Add a single optional field for the terminal width, set once at the render root:

```rust
pub struct Layout {
    // ... existing fields ...

    /// The terminal width in characters, set at the render root.
    /// When `None`, components use a default (typically 80) or query Terminal.
    /// This value is set ONCE at render time and propagated unchanged to children.
    pub terminal_width: Option<u32>,
}
```

**Why this is acceptable**: `terminal_width` is not a constraint - it's the *context* in which constraints are evaluated. It's set once at the top of the render tree and never modified. Components don't set this when building layouts; it's injected at render time.

### Part 2: Add `available_width()` Method

Add a method that evaluates the lazy constraints:

```rust
impl Layout {
    /// Calculate the available content width by evaluating margin constraints
    /// against the terminal width.
    ///
    /// Returns `None` if terminal_width is not set.
    pub fn available_width(&self) -> Option<u32> {
        let term_width = self.terminal_width?;
        let left = Self::resolve_margin(&self.left_margin, term_width);
        let right = Self::resolve_margin(&self.right_margin, term_width);
        Some(term_width.saturating_sub(left).saturating_sub(right))
    }

    /// Create a child layout with additional left margin.
    ///
    /// This is how parents constrain children - by increasing margins,
    /// not by setting computed widths.
    pub fn with_additional_left_margin(&self, chars: u32) -> Layout {
        let new_left = match &self.left_margin {
            Margin::None => Margin::Chars(chars),
            Margin::Chars(existing) => Margin::Chars(existing + chars),
            Margin::Percent(pct) => {
                // Convert percent to chars and add
                // Requires terminal_width, fall back to just chars if not set
                if let Some(tw) = self.terminal_width {
                    let existing_chars = ((tw as f32) * pct / 100.0).round() as u32;
                    Margin::Chars(existing_chars + chars)
                } else {
                    Margin::Chars(chars)
                }
            }
        };
        Layout {
            left_margin: new_left,
            ..self.clone()
        }
    }
}
```

### Part 3: Distinguish Block-Level vs Inline Content

The "double bullet" problem requires the PARENT to know whether a child is block-level (renders its own structure) or inline (just text to decorate).

**Option A: Trait method on Renderable**

```rust
pub trait Renderable: std::fmt::Debug {
    fn render(&self, layout: Option<&Layout>) -> String;
    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String;

    /// Returns true if this component renders as a block-level element
    /// with its own structure (lists, block quotes, etc.).
    ///
    /// Block-level components should NOT have parent decorations
    /// (bullets, numbers) prepended to them.
    fn is_block_level(&self) -> bool {
        false  // Default: inline
    }
}
```

**Option B: Extend RenderableContent enum**

```rust
pub enum RenderableContent {
    /// Plain text - parent may add decorations
    String(String),
    /// Inline component - parent may add decorations
    Inline(Arc<dyn Renderable>),
    /// Block-level component - parent should NOT add decorations,
    /// only indent via layout constraints
    Block(Arc<dyn Renderable>),
}
```

**Option C: Marker trait**

```rust
/// Marker trait for block-level components.
/// Components implementing this will not have parent decorations applied.
pub trait BlockLevel: Renderable {}

impl BlockLevel for UnorderedList {}
impl BlockLevel for OrderedList {}
impl BlockLevel for BlockQuote {}
```

**Recommendation**: Option A (trait method) is simplest and most flexible. It doesn't require changing the RenderableContent enum or adding marker traits.

### Part 4: Update List Rendering Logic

```rust
impl UnorderedList {
    fn render_content(&self, term: Option<&Terminal>, layout: Option<&Layout>) -> String {
        let layout = layout.cloned().unwrap_or_default();
        let bullet_width = visible_width(&self.bullet) as u32;

        let mut result = String::new();

        for (i, item) in self.items.iter().enumerate() {
            match item {
                RenderableContent::String(s) => {
                    // Inline content: add bullet + text
                    result.push_str(&self.bullet);
                    result.push_str(s);
                }
                RenderableContent::Component(component) => {
                    if component.is_block_level() {
                        // Block-level: NO bullet, create indented child layout
                        let child_layout = layout.with_additional_left_margin(bullet_width);
                        let content = if let Some(t) = term {
                            component.fallback_render(t, Some(&child_layout))
                        } else {
                            component.render(Some(&child_layout))
                        };
                        result.push_str(&content);
                    } else {
                        // Inline component: add bullet + rendered content
                        result.push_str(&self.bullet);
                        let content = if let Some(t) = term {
                            component.fallback_render(t, Some(&layout))
                        } else {
                            component.render(Some(&layout))
                        };
                        result.push_str(&content);
                    }
                }
            }

            if i < self.items.len() - 1 {
                result.push('\n');
            }
        }

        result
    }
}
```

---

## How Width Constraints Propagate

The key insight: **width constraints propagate through increased margins, not computed values**.

```
Terminal width: 80
Top-level Layout: left_margin = 0, right_margin = 0
  → available_width() = 80

Parent list renders with bullet "• " (width 2)
  → Creates child layout: left_margin = 2
  → Child's available_width() = 78

Nested list has bullet "• " (width 2)
  → Creates grandchild layout: left_margin = 4
  → Grandchild's available_width() = 76
```

Each level increases the left margin by its decoration width. The lazy constraint system handles the rest.

---

## Detailed Behavior Specification

### Nested UnorderedList Rendering

**Input:**

```rust
let inner = UnorderedList::new(vec!["Nested A", "Nested B"]);
let outer = UnorderedList::from(vec![
    RenderableContent::String("First item".into()),
    RenderableContent::String("Second item".into()),
    RenderableContent::Component(Arc::new(inner)),  // Block-level
]);
```

**Render flow:**

1. Outer list iterates items
2. "First item" → String → prepend bullet → `"• First item"`
3. "Second item" → String → prepend bullet → `"• Second item"`
4. Inner list → Component → `is_block_level() = true` → NO bullet
   - Create child layout with `left_margin += 2`
   - Inner list renders with that layout
   - Inner list adds ITS bullets: `"• Nested A"`, `"• Nested B"`
   - But child layout's margin indents these

**Expected Output:**

```
• First item
• Second item
  • Nested A
  • Nested B
```

### Mixed Inline and Block Content

**Input:**

```rust
let prose = Prose::new("<b>Important</b> note");  // Inline
let nested_list = UnorderedList::new(vec!["Sub A", "Sub B"]);  // Block

let outer = UnorderedList::from(vec![
    RenderableContent::Component(Arc::new(prose)),
    RenderableContent::Component(Arc::new(nested_list)),
]);
```

**Expected Output:**

```
• Important note
  • Sub A
  • Sub B
```

Prose is inline (gets bullet), nested list is block (no bullet, indented).

---

## Implementation Plan

### Stage 1: Layout Changes (Non-Breaking)

1. Add `terminal_width: Option<u32>` field with default `None`
2. Add `available_width(&self) -> Option<u32>` method
3. Add `with_additional_left_margin(&self, chars: u32) -> Layout` method
4. Update `Default` impl to include new field
5. Add unit tests

### Stage 2: Renderable Trait Extension

1. Add `fn is_block_level(&self) -> bool { false }` with default impl
2. Implement `is_block_level() -> true` for:
   - `UnorderedList`
   - `OrderedList`
   - `BlockQuote`
   - `Section`
   - `Table`

### Stage 3: List Component Updates

1. Update `UnorderedList::render_content()` to check `is_block_level()`
2. Update `OrderedList::render_content()` similarly
3. Pass child layout with increased margin for block children
4. Add comprehensive tests for nested scenarios

### Stage 4: Other Components

1. Update `BlockQuote` to pass constrained layout to children
2. Update `Section` similarly
3. Ensure `Table` cell rendering handles block content

---

## Breaking Change Assessment

### Trait Signature: Minor Addition

```rust
// Before
pub trait Renderable: std::fmt::Debug {
    fn render(&self, layout: Option<&Layout>) -> String;
    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String;
}

// After (with default impl)
pub trait Renderable: std::fmt::Debug {
    fn render(&self, layout: Option<&Layout>) -> String;
    fn fallback_render(&self, term: &Terminal, layout: Option<&Layout>) -> String;
    fn is_block_level(&self) -> bool { false }  // Default preserves existing behavior
}
```

**Non-breaking**: Default implementation returns `false`, preserving existing behavior.

### Layout Struct: Additive Only

New field has default that preserves existing behavior:

- `terminal_width: None` (existing behavior - use default or Terminal query)

**Non-breaking**: Existing code continues to work.

---

## Test Cases

### Block-Level Detection

```rust
#[test]
fn test_unordered_list_is_block_level() {
    let list = UnorderedList::new(vec!["item"]);
    assert!(list.is_block_level());
}

#[test]
fn test_prose_is_not_block_level() {
    let prose = Prose::new("text");
    assert!(!prose.is_block_level());
}
```

### Nested List Without Double Bullets

```rust
#[test]
fn test_nested_list_no_double_bullets() {
    let inner = UnorderedList::new(vec!["Nested"]);
    let outer = UnorderedList::from(vec![
        RenderableContent::Component(Arc::new(inner)),
    ]);

    let result = outer.render(None);

    // Should NOT contain "• • " (double bullet)
    assert!(!result.contains("• • "));
    // Should contain properly indented nested bullet
    assert!(result.contains("  • Nested"));
}
```

### Width Constraint via Margins

```rust
#[test]
fn test_available_width_calculation() {
    let layout = Layout {
        terminal_width: Some(80),
        left_margin: Margin::Chars(10),
        right_margin: Margin::Chars(10),
        ..Layout::default()
    };

    assert_eq!(layout.available_width(), Some(60));
}

#[test]
fn test_child_layout_margin_accumulation() {
    let parent = Layout {
        terminal_width: Some(80),
        left_margin: Margin::Chars(4),
        ..Layout::default()
    };

    let child = parent.with_additional_left_margin(2);

    assert_eq!(child.left_margin, Margin::Chars(6));
    assert_eq!(child.available_width(), Some(74)); // 80 - 6 - 0
}
```

---

## Alternative Designs Considered

### 1. RenderContext Enum in Layout

Store nesting depth and consumed width in Layout.

**Rejected because:**

- Violates lazy constraint principle
- Mixes declarative constraints with computed state
- Makes Layout brittle and component-specific

### 2. Separate NestedList Components

Create `NestedUnorderedList` that never renders bullets.

**Rejected because:**

- Doubles API surface
- Users must know when to use which
- Doesn't solve width propagation

### 3. Global/Thread-Local Nesting State

Track nesting depth in thread-local storage.

**Rejected because:**

- Not composable
- Race conditions in async contexts
- Violates functional rendering principles

---

## Open Questions

1. **Should `terminal_width` be required at render time?**
   - Current: Optional, fall back to 80 or Terminal query
   - Alternative: Required, forcing explicit width at render root

2. **How should Option B (RenderableContent variants) interact with Option A (trait method)?**
   - If we go with Option A only, no change to RenderableContent
   - If we want both, the enum variant could override the trait method

3. **Should there be a `is_inline()` method as well, or just `is_block_level()`?**
   - Current: Just `is_block_level()`, default is inline
   - Alternative: Both methods for clarity

---

## Conclusion

This design preserves the lazy constraint architecture of `Layout` while solving the recursive rendering problems:

1. **`terminal_width`**: Injected once at render root, propagated unchanged
2. **`available_width()`**: Evaluates lazy constraints on demand
3. **`with_additional_left_margin()`**: Propagates constraints to children
4. **`is_block_level()`**: Lets parents distinguish decoration targets

The solution keeps component-specific concerns (nesting depth, prefix width) out of Layout, maintains backward compatibility, and provides a clean extension point for future recursive components.

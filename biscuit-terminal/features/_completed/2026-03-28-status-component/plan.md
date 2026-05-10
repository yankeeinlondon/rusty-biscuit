# Status Component - Implementation Plan

## Overview

Add a `Status` renderable component to `biscuit-terminal` following the existing `Todo` component pattern. Status reports validation/action-item state using themed icons (Circular, Rounded, Timeline) with Tailwind-based colorization and light/dark mode support.

## Key Differences from Todo

| Aspect | Todo | Status |
|--------|------|--------|
| States | Open, InProgress, Completed, Cancelled, Blocked | NotStarted, Active, Success, Failure, Warning, Info |
| Themes | Single theme | 3 themes: Circular, Rounded, Timeline |
| Icon style | GFM-compatible `[x]` fallbacks | Plain Unicode fallbacks (no square brackets) |
| Colors | BasicColor (green, red) | Tailwind colors with light/dark variants |
| Color toggle | N/A | `.no_color_icons()` builder method |

## Phase 1: Core Types and Structs

**File:** `biscuit-terminal/lib/src/components/status.rs`

### 1.1 StatusState Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusState {
    NotStarted,
    Active,
    Success,
    Failure,
    Warning,
    Info,
}
```

### 1.2 StatusTheme Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StatusTheme {
    #[default]
    Circular,
    Rounded,
    Timeline,
}
```

### 1.3 StatusIconDef Struct

Represents the icon definition for one theme+state combination:

```rust
struct StatusIconDef {
    nerd: &'static str,       // Nerd Font Unicode character
    fallback: &'static str,   // Plain Unicode fallback
    color: Tailwind,          // Primary Tailwind color
    color_alt: Option<Tailwind>, // Light-mode alternative (for "/" colors like gray-600/400)
}
```

### 1.4 Static Lookup Table

Use `LazyLock<HashMap<(StatusTheme, StatusState), StatusIconDef>>` following the `TODO_CHAR_LOOKUP` pattern, populated from the spec's theme matrix (18 entries: 3 themes x 6 states).

### 1.5 Status Struct

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    state: StatusState,
    theme: StatusTheme,
    description: String,
    color_icons: bool,        // default: true, toggled by .no_color_icons()
    #[serde(skip)]
    layout: Layout,
}
```

Builder methods:
- `Status::new(desc)` - Creates with `NotStarted` state, `Circular` theme
- `.state(StatusState)` - Set state
- `.theme(StatusTheme)` - Set theme
- `.no_color_icons()` - Disable icon colorization

## Phase 2: Rendering Logic

### 2.1 Private `to_terminal(&self, term: &Terminal) -> String`

Follow the same pattern as `Todo::to_terminal()`:

1. Look up `StatusIconDef` from the static table using `(self.theme, self.state)`
2. Determine icon character:
   - `term.is_nerd_font == Some(true)` -> use `icon_def.nerd`
   - Otherwise -> use `icon_def.fallback`
3. Colorize icon (when `self.color_icons` AND `term.color_depth != ColorDepth::None`):
   - Check `Terminal::color_mode()` for light/dark
   - If `Light` and `color_alt.is_some()` -> use alt color
   - Otherwise -> use primary color
   - Apply via `TailwindColorWrapper(color).fallback_render(icon, term)` for color-depth-aware rendering
4. If `ColorDepth::None` or `!self.color_icons` -> render plain icon without color
5. Format as `"{icon} {description}"`

### 2.2 Renderable Trait Implementation

Mirror `Todo`'s implementation exactly:

```rust
impl Renderable for Status {
    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let content = self.to_terminal(term);
        self.layout.apply_layout(&content, width)
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        let content = self.to_terminal(&term);
        self.layout.apply_layout(&content, width)
    }

    fn layout(&self) -> &Layout { &self.layout }
    fn layout_mut(&mut self) -> &mut Layout { &mut self.layout }
    fn as_any(&self) -> &dyn Any { self }
}
```

## Phase 3: Module Registration and Exports

### 3.1 Register Module

Add `pub mod status;` to `biscuit-terminal/lib/src/components/mod.rs`.

### 3.2 Prelude Export

Add to `biscuit-terminal/lib/src/prelude.rs`:

```rust
pub use crate::components::status::{Status, StatusState, StatusTheme};
```

## Phase 4: Tests

### 4.1 Unit Tests (in `status.rs`)

Test helpers:
- `no_color_terminal()` - `ColorDepth::None`, `is_nerd_font: Some(false)`
- `color_terminal()` - `ColorDepth::TrueColor`, `is_nerd_font: Some(false)`
- `nerd_terminal()` - `ColorDepth::TrueColor`, `is_nerd_font: Some(true)`

Test cases:
1. **No-color rendering** - all 6 states with Circular theme produce correct fallback icons without ANSI codes
2. **Color rendering** - states produce output with ANSI escape codes
3. **Nerd font rendering** - uses nerd font characters when detected
4. **Theme variations** - Rounded and Timeline themes produce different icons
5. **no_color_icons builder** - disables icon colorization even when terminal supports color
6. **Default state** - `Status::new()` creates `NotStarted` with `Circular` theme
7. **Builder chaining** - `.state()`, `.theme()`, `.no_color_icons()` work fluently

## Phase 5: Build Verification

1. `cargo build -p biscuit-terminal`
2. `cargo test -p biscuit-terminal`
3. `cargo clippy -p biscuit-terminal`

## Nerd Font Icon Reference

All codes from the spec, as Rust Unicode escapes:

| Theme | State | Nerd Code | Rust Escape |
|-------|-------|-----------|-------------|
| Circular | NotStarted | f4aa | `\u{f4aa}` |
| Circular | Active | f0ec2 | `\u{f0ec2}` |
| Circular | Success | f05e0 | `\u{f05e0}` |
| Circular | Failure | f057 | `\u{f057}` |
| Circular | Warning | f0028 | `\u{f0028}` |
| Circular | Info | f449 | `\u{f449}` |
| Rounded | NotStarted | ea72 | `\u{ea72}` |
| Rounded | Active | f1500 | `\u{f1500}` |
| Rounded | Success | f14a | `\u{f14a}` |
| Rounded | Failure | f136e | `\u{f136e}` |
| Rounded | Warning | f0af | `\u{f0af}` |
| Rounded | Info | f0bd4 | `\u{f0bd4}` |
| Timeline | NotStarted | f0bd2 | `\u{f0bd2}` |
| Timeline | Active | f0bd1 | `\u{f0bd1}` |
| Timeline | Success | f1532 | `\u{f1532}` |
| Timeline | Failure | f1537 | `\u{f1537}` |
| Timeline | Warning | f0f95 | `\u{f0f95}` |
| Timeline | Info | f0bd4 | `\u{f0bd4}` |

## Color Mapping

| Spec Color | Primary Tailwind | Alt (Light Mode) |
|------------|-----------------|------------------|
| gray-500 | `Tailwind::Gray500` | - |
| gray-600/400 | `Tailwind::Gray600` | `Tailwind::Gray400` |
| green-500 | `Tailwind::Green500` | - |
| red-500 | `Tailwind::Red500` | - |
| orange-500 | `Tailwind::Orange500` | - |
| blue-500 | `Tailwind::Blue500` | - |

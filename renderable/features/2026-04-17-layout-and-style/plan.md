# Layout Primitive Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the three forked layout representations (`renderable::layout::Layout`, `tree::LayoutHints`, darkmatter's `DarkmatterPage` page types) with one `Layout` primitive that every block-level component declares and the tree renderers apply across Terminal, Browser, and Markdown.

**Architecture:** A new `renderable::layout` module defines `Length`, `TargetValue<T>`, `Margin`, and `Layout`. `Layout` rides on block `RenderNode`s via a typed `NodeAttrs` accessor (replacing `LayoutHints`). The Terminal and Browser tree renderers consume it; the Markdown renderer deliberately ignores it. `biscuit-terminal` keeps terminal-only application in `LayoutTerminalExt`; `darkmatter`'s `DarkmatterPage` stops re-inventing the contract.

**Tech Stack:** Rust 2024 edition, `serde`, `proptest`, the rusty-biscuit monorepo workspace. Build/test always with `-p <pkg>` (never a bare workspace build).

**Reference:** `renderable/features/2026-04-17-layout-and-style/spec.md` (Spec A).

---

## File Structure

**`renderable` crate — new module `src/layout/`** (today `src/layout.rs`; promote to a directory):

- `src/layout/mod.rs` — re-exports; the `Layout` struct; `LayoutError`.
- `src/layout/length.rs` — `Length`.
- `src/layout/target_value.rs` — `TargetValue<T>`.
- `src/layout/margin.rs` — `Margin` + `Alignment`.

**`renderable` crate — modified:**

- `src/target.rs` — `RenderTarget`: drop `Ast`, add `MarkdownPlus`, derive serde/ord.
- `src/wrap_policy.rs`, `src/stylesheet/value.rs` — add `serde` derives.
- `src/tree/attrs.rs` — delete `LayoutHints`; add `NodeAttrs::layout`/`set_layout`.
- `src/tree/mod.rs` — `TreeRenderable::tree_layout_hints` retyped.
- `src/tree/validate.rs` — block-only layout rule.
- `src/tree/render/browser.rs`, `.../markdown.rs` — consume / ignore `Layout`.

**`biscuit-terminal` crate — modified:**

- `lib/src/utils/layout.rs` — `LayoutTerminalExt` adapted to the new `Layout`.
- `lib/src/render_tree/options.rs`, `.../render.rs` — terminal renderer applies `Layout`.
- The 6 terminal components' `render_tree_node` impls.

**`darkmatter` crate — modified:**

- `lib/src/layout/` — `DarkmatterPage` maps page margins onto `Layout`; `From`/`TryFrom` deprecation conversions on `PageMargin`/`PageFill`/`PageAlignment`.
- `lib/src/markdown/yaml_block.rs` — `YamlBlock::render_tree_node`.

---

## Phase 1 — Core Types (`renderable`)

### Task 1: Make supporting types serde-ready; fix `RenderTarget`

**Files:**
- Modify: `renderable/src/target.rs`
- Modify: `renderable/src/wrap_policy.rs`
- Modify: `renderable/src/stylesheet/value.rs`
- Test: `renderable/src/target.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test** in `renderable/src/target.rs`'s test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_target_has_no_ast_and_has_markdown_plus() {
        // MarkdownPlus exists; Ast is gone.
        let t = RenderTarget::MarkdownPlus;
        assert_eq!(t, RenderTarget::MarkdownPlus);
    }

    #[test]
    fn render_target_serde_roundtrip_snake_case() {
        let json = serde_json::to_string(&RenderTarget::MarkdownPlus).unwrap();
        assert_eq!(json, "\"markdown_plus\"");
        let back: RenderTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(back, RenderTarget::MarkdownPlus);
    }

    #[test]
    fn render_target_is_ord() {
        let mut v = vec![RenderTarget::Terminal, RenderTarget::Browser];
        v.sort();
        assert_eq!(v, vec![RenderTarget::Browser, RenderTarget::Terminal]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p renderable target::tests`
Expected: FAIL — `MarkdownPlus` not found / `Ast` still present / not `Ord`.

- [ ] **Step 3: Update `RenderTarget`**

In `renderable/src/target.rs`, replace the enum and derives with:

```rust
use serde::{Deserialize, Serialize};

/// A concrete render target.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RenderTarget {
    Markdown,
    MarkdownPlus,
    Terminal,
    Browser,
}
```

The legacy `Ast` variant is removed (the `AstRenderable` trait is already gone). Search the crate for any remaining `RenderTarget::Ast` (`grep -rn "RenderTarget::Ast" renderable/src`) and delete those arms — the explore pass found none, but verify.

- [ ] **Step 4: Add serde derives to `WordWrap` and `CssSizing`/`CssUnit`**

In `renderable/src/wrap_policy.rs`, add `Serialize, Deserialize` to the `WordWrap` derive list and `#[serde(rename_all = "snake_case")]`.

In `renderable/src/stylesheet/value.rs`, add `Serialize, Deserialize` to the `CssSizing` and `CssUnit` derive lists, with `#[serde(rename_all = "snake_case")]` on each. (These are plain data enums; the derive is mechanical.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p renderable target::tests`
Expected: PASS. Then `cargo build -p renderable` — Expected: builds.

- [ ] **Step 6: Commit**

```bash
git add renderable/src/target.rs renderable/src/wrap_policy.rs renderable/src/stylesheet/value.rs
git commit -m "refactor(renderable): drop RenderTarget::Ast, add MarkdownPlus, make layout deps serde-ready"
```

---

### Task 2: `Length` and `LayoutError`

**Files:**
- Create: `renderable/src/layout/length.rs`
- Modify: `renderable/src/layout.rs` → convert to `renderable/src/layout/mod.rs` (see Step 3)
- Test: in `length.rs` `#[cfg(test)]`

- [ ] **Step 1: Promote `layout.rs` to a module directory**

```bash
mkdir renderable/src/layout
git mv renderable/src/layout.rs renderable/src/layout/mod.rs
```

(The old contents of `mod.rs` are rewritten in Task 5; leave them in place for now so the crate keeps compiling.)

- [ ] **Step 2: Write the failing test** — create `renderable/src/layout/length.rs`:

```rust
//! The `Length` layout value.

use serde::{Deserialize, Serialize};

use crate::stylesheet::CssSizing;

/// A layout length.
///
/// `Zero`, `Ch`, and `Percent` are the **universal units** — valid on every
/// render target. `Css` carries a target-native value and is valid only
/// inside the per-target branch of a [`TargetValue`](super::TargetValue).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    /// Zero — unit-independent.
    Zero,
    /// Whole cells. Columns on horizontal sides, rows on vertical sides.
    Ch(u32),
    /// Percentage of the available width, `0.0..=100.0`.
    Percent(f32),
    /// A target-native CSS length. Only valid in a per-target branch.
    Css(CssSizing),
}

/// An error constructing or validating a layout value.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LayoutError {
    /// A percentage outside `0.0..=100.0`, or non-finite.
    #[error("invalid percentage `{0}`: must be a finite value in 0.0..=100.0")]
    InvalidPercent(f32),
    /// A `Length::Css` value used in a `TargetValue::Universal` branch.
    #[error(
        "non-universal unit in a universal value: `{0}`; use a per-target \
         map (e.g. {{ browser: ..., terminal: ... }}) for target-native units"
    )]
    NonUniversalUnit(String),
    /// An empty `TargetValue::PerTarget` map.
    #[error("per-target value map is empty")]
    EmptyPerTarget,
}

impl Length {
    /// Zero length.
    pub fn zero() -> Length {
        Length::Zero
    }

    /// `n` whole cells.
    pub fn ch(n: u32) -> Length {
        Length::Ch(n)
    }

    /// A validated percentage in `0.0..=100.0`.
    ///
    /// ## Errors
    /// [`LayoutError::InvalidPercent`] when `pct` is non-finite or out of range.
    pub fn percent(pct: f32) -> Result<Length, LayoutError> {
        if pct.is_finite() && (0.0..=100.0).contains(&pct) {
            Ok(Length::Percent(pct))
        } else {
            Err(LayoutError::InvalidPercent(pct))
        }
    }

    /// A target-native CSS length.
    pub fn css(sizing: CssSizing) -> Length {
        Length::Css(sizing)
    }

    /// Whether this length uses a universal unit (valid on every target).
    pub fn is_universal(&self) -> bool {
        !matches!(self, Length::Css(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_rejects_out_of_range() {
        assert_eq!(Length::percent(150.0), Err(LayoutError::InvalidPercent(150.0)));
        assert_eq!(Length::percent(-1.0), Err(LayoutError::InvalidPercent(-1.0)));
        assert!(matches!(
            Length::percent(f32::NAN),
            Err(LayoutError::InvalidPercent(_))
        ));
    }

    #[test]
    fn percent_accepts_in_range() {
        assert_eq!(Length::percent(0.0), Ok(Length::Percent(0.0)));
        assert_eq!(Length::percent(100.0), Ok(Length::Percent(100.0)));
    }

    #[test]
    fn is_universal_is_false_only_for_css() {
        assert!(Length::zero().is_universal());
        assert!(Length::ch(4).is_universal());
        assert!(Length::Percent(50.0).is_universal());
        assert!(!Length::css(CssSizing::px(8.0)).is_universal());
    }

    #[test]
    fn length_serde_roundtrip() {
        for value in [Length::Zero, Length::Ch(4), Length::Percent(50.0)] {
            let json = serde_json::to_string(&value).unwrap();
            let back: Length = serde_json::from_str(&json).unwrap();
            assert_eq!(value, back);
        }
    }
}
```

- [ ] **Step 3: Wire the module** — in `renderable/src/layout/mod.rs` add near the top:

```rust
mod length;
pub use length::{Length, LayoutError};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p renderable layout::length`
Expected: PASS (all four tests).

- [ ] **Step 5: Commit**

```bash
git add renderable/src/layout/
git commit -m "feat(renderable): add Length layout value and LayoutError"
```

---

### Task 3: `TargetValue<T>` with target resolution

**Files:**
- Create: `renderable/src/layout/target_value.rs`
- Modify: `renderable/src/layout/mod.rs`
- Test: in `target_value.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the failing test + type** — create `renderable/src/layout/target_value.rs`:

```rust
//! Per-target layout values.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::layout::Length;
use crate::layout::length::LayoutError;
use crate::target::RenderTarget;

/// A layout value that is either universal or specified per render target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetValue<T> {
    /// One value for every target. Universal-units only (for `Length`).
    Universal(T),
    /// Per-target values. Non-empty; each entry may use that target's
    /// native units. A target not named here does not receive the property.
    PerTarget(BTreeMap<RenderTarget, T>),
}

impl<T> TargetValue<T> {
    /// A universal value.
    pub fn universal(value: T) -> TargetValue<T> {
        TargetValue::Universal(value)
    }

    /// Resolve the value for `target`.
    ///
    /// `Universal` always resolves. `PerTarget` looks up `target`; a
    /// `MarkdownPlus` lookup falls back to the `Markdown` entry. Returns
    /// `None` when a `PerTarget` map names neither.
    pub fn resolve(&self, target: RenderTarget) -> Option<&T> {
        match self {
            TargetValue::Universal(value) => Some(value),
            TargetValue::PerTarget(map) => map.get(&target).or_else(|| {
                if target == RenderTarget::MarkdownPlus {
                    map.get(&RenderTarget::Markdown)
                } else {
                    None
                }
            }),
        }
    }
}

impl TargetValue<Length> {
    /// Validate this length value.
    ///
    /// ## Errors
    /// - [`LayoutError::NonUniversalUnit`] — a `Length::Css` in the
    ///   `Universal` branch.
    /// - [`LayoutError::EmptyPerTarget`] — an empty `PerTarget` map.
    /// - [`LayoutError::InvalidPercent`] — a non-finite / out-of-range percent.
    pub fn validate(&self) -> Result<(), LayoutError> {
        match self {
            TargetValue::Universal(length) => {
                check_percent(length)?;
                if !length.is_universal() {
                    return Err(LayoutError::NonUniversalUnit(format!("{length:?}")));
                }
                Ok(())
            }
            TargetValue::PerTarget(map) => {
                if map.is_empty() {
                    return Err(LayoutError::EmptyPerTarget);
                }
                for length in map.values() {
                    check_percent(length)?;
                }
                Ok(())
            }
        }
    }
}

fn check_percent(length: &Length) -> Result<(), LayoutError> {
    if let Length::Percent(pct) = length
        && !(pct.is_finite() && (0.0..=100.0).contains(pct))
    {
        return Err(LayoutError::InvalidPercent(*pct));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stylesheet::CssSizing;

    #[test]
    fn universal_resolves_for_every_target() {
        let v = TargetValue::universal(Length::ch(2));
        for t in [
            RenderTarget::Terminal,
            RenderTarget::Browser,
            RenderTarget::Markdown,
            RenderTarget::MarkdownPlus,
        ] {
            assert_eq!(v.resolve(t), Some(&Length::ch(2)));
        }
    }

    #[test]
    fn per_target_resolves_named_targets_only() {
        let mut map = BTreeMap::new();
        map.insert(RenderTarget::Browser, Length::css(CssSizing::px(8.0)));
        map.insert(RenderTarget::Terminal, Length::ch(2));
        let v = TargetValue::PerTarget(map);
        assert_eq!(v.resolve(RenderTarget::Terminal), Some(&Length::ch(2)));
        assert_eq!(v.resolve(RenderTarget::Markdown), None);
    }

    #[test]
    fn markdown_plus_falls_back_to_markdown() {
        let mut map = BTreeMap::new();
        map.insert(RenderTarget::Markdown, Length::ch(1));
        let v = TargetValue::PerTarget(map);
        assert_eq!(v.resolve(RenderTarget::MarkdownPlus), Some(&Length::ch(1)));
    }

    #[test]
    fn validate_rejects_css_in_universal() {
        let v = TargetValue::universal(Length::css(CssSizing::rem(1.0)));
        assert!(matches!(v.validate(), Err(LayoutError::NonUniversalUnit(_))));
    }

    #[test]
    fn validate_rejects_empty_per_target() {
        let v: TargetValue<Length> = TargetValue::PerTarget(BTreeMap::new());
        assert_eq!(v.validate(), Err(LayoutError::EmptyPerTarget));
    }

    #[test]
    fn validate_accepts_css_in_per_target() {
        let mut map = BTreeMap::new();
        map.insert(RenderTarget::Browser, Length::css(CssSizing::rem(1.0)));
        assert!(TargetValue::PerTarget(map).validate().is_ok());
    }
}
```

- [ ] **Step 2: Wire the module** — in `renderable/src/layout/mod.rs`:

```rust
mod target_value;
pub use target_value::TargetValue;
```

- [ ] **Step 3: Run test to verify it fails then passes**

Run: `cargo test -p renderable layout::target_value`
Expected: PASS (six tests). If `&&`-let chains error, the crate is on an edition/toolchain without let-chains — rewrite `check_percent` with nested `if let`.

- [ ] **Step 4: Commit**

```bash
git add renderable/src/layout/
git commit -m "feat(renderable): add TargetValue with per-target resolution and validation"
```

---

### Task 4: `Margin` and `Alignment`

**Files:**
- Create: `renderable/src/layout/margin.rs`
- Modify: `renderable/src/layout/mod.rs`
- Test: in `margin.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the type + tests** — create `renderable/src/layout/margin.rs`:

```rust
//! Margin box and alignment for [`Layout`](super::Layout).

use serde::{Deserialize, Serialize};

use crate::layout::{Length, TargetValue};
use crate::layout::length::LayoutError;

/// Horizontal alignment of a block within its parent's available width.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Alignment {
    /// Left-aligned (default).
    #[default]
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

/// A four-sided margin box. Each side is a [`TargetValue<Length>`].
///
/// All sides accept the same `Ch` / `Percent` / `Zero` units; the browser
/// renderer lowers vertical sides (`top` / `bottom`) to `lh` automatically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Margin {
    pub top: TargetValue<Length>,
    pub right: TargetValue<Length>,
    pub bottom: TargetValue<Length>,
    pub left: TargetValue<Length>,
}

impl Default for Margin {
    fn default() -> Margin {
        Margin {
            top: TargetValue::universal(Length::Zero),
            right: TargetValue::universal(Length::Zero),
            bottom: TargetValue::universal(Length::Zero),
            left: TargetValue::universal(Length::Zero),
        }
    }
}

impl Margin {
    /// A margin with all four sides set to the same universal length.
    pub fn all(length: Length) -> Margin {
        Margin {
            top: TargetValue::universal(length.clone()),
            right: TargetValue::universal(length.clone()),
            bottom: TargetValue::universal(length.clone()),
            left: TargetValue::universal(length),
        }
    }

    /// A margin with left + right set to `length`, top + bottom zero.
    pub fn x(length: Length) -> Margin {
        Margin {
            right: TargetValue::universal(length.clone()),
            left: TargetValue::universal(length),
            ..Margin::default()
        }
    }

    /// A margin with top + bottom set to `length`, left + right zero.
    pub fn y(length: Length) -> Margin {
        Margin {
            top: TargetValue::universal(length.clone()),
            bottom: TargetValue::universal(length),
            ..Margin::default()
        }
    }

    /// Validate every side.
    ///
    /// ## Errors
    /// Propagates the first [`LayoutError`] from any side's
    /// [`TargetValue::validate`].
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.top.validate()?;
        self.right.validate()?;
        self.bottom.validate()?;
        self.left.validate()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_zero() {
        let m = Margin::default();
        assert_eq!(m.left, TargetValue::universal(Length::Zero));
        assert_eq!(m.top, TargetValue::universal(Length::Zero));
    }

    #[test]
    fn x_sets_only_horizontal() {
        let m = Margin::x(Length::ch(4));
        assert_eq!(m.left, TargetValue::universal(Length::ch(4)));
        assert_eq!(m.right, TargetValue::universal(Length::ch(4)));
        assert_eq!(m.top, TargetValue::universal(Length::Zero));
    }

    #[test]
    fn validate_propagates_errors() {
        let m = Margin {
            left: TargetValue::universal(Length::Percent(150.0)),
            ..Margin::default()
        };
        assert!(m.validate().is_err());
    }

    #[test]
    fn alignment_default_is_left() {
        assert_eq!(Alignment::default(), Alignment::Left);
    }
}
```

- [ ] **Step 2: Wire the module** — in `renderable/src/layout/mod.rs`:

```rust
mod margin;
pub use margin::{Alignment, Margin};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p renderable layout::margin`
Expected: PASS (four tests).

- [ ] **Step 4: Commit**

```bash
git add renderable/src/layout/
git commit -m "feat(renderable): add Margin box and Alignment"
```

---

### Task 5: The `Layout` struct

**Files:**
- Modify: `renderable/src/layout/mod.rs` (replace the legacy `Layout`/`Margin`/`MaxWidth`/`RowFill` body)
- Test: in `mod.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the new `Layout`** — replace the entire body of `renderable/src/layout/mod.rs` with module wiring plus:

```rust
//! Target-agnostic layout configuration.

mod length;
mod margin;
mod target_value;

pub use length::{Length, LayoutError};
pub use margin::{Alignment, Margin};
pub use target_value::TargetValue;

pub use crate::wrap_policy::WordWrap;

use serde::{Deserialize, Serialize};

/// A block-level component's relationship to its parent: margins, alignment
/// within the parent, max-width, and content wrapping.
///
/// Inline components carry no `Layout`. Appearance (background, fill) is a
/// `Style` concern and is not represented here.
///
/// ## Serialized shape
///
/// `Layout` rides on `NodeAttrs` and serializes with the render tree. A node
/// with `margin-left: 2ch` and `margin-right` differing per target:
///
/// ```json
/// {
///   "margin": {
///     "top": { "universal": "zero" },
///     "right": { "per_target": { "browser": { "css": "5em" },
///                                "terminal": { "ch": 5 } } },
///     "bottom": { "universal": "zero" },
///     "left": { "universal": { "ch": 2 } }
///   },
///   "alignment": "left",
///   "max_width": null,
///   "word_wrap": "none"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Layout {
    /// Outer margins.
    pub margin: Margin,
    /// Alignment within the parent's available width.
    pub alignment: Alignment,
    /// Optional cap on content width.
    pub max_width: Option<TargetValue<Length>>,
    /// Content-wrapping policy.
    pub word_wrap: WordWrap,
}

impl Layout {
    /// Validate every length-valued field.
    ///
    /// ## Errors
    /// Propagates the first [`LayoutError`] from `margin` or `max_width`.
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.margin.validate()?;
        if let Some(max_width) = &self.max_width {
            max_width.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_is_zero_margins_left_aligned() {
        let layout = Layout::default();
        assert_eq!(layout.margin, Margin::default());
        assert_eq!(layout.alignment, Alignment::Left);
        assert!(layout.max_width.is_none());
    }

    #[test]
    fn validate_rejects_bad_max_width() {
        let layout = Layout {
            max_width: Some(TargetValue::universal(Length::Percent(200.0))),
            ..Layout::default()
        };
        assert!(layout.validate().is_err());
    }

    #[test]
    fn layout_serde_roundtrip() {
        let layout = Layout {
            margin: Margin::x(Length::ch(2)),
            alignment: Alignment::Center,
            ..Layout::default()
        };
        let json = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(layout, back);
    }
}
```

`WordWrap`'s `Default` is `WrapProse(Some(8), None)`; if `Layout` should default to `WordWrap::None`, implement `Default` manually instead of deriving. **Decision:** match the spec's "single presentational tier" intent — derive `Default` and accept `WordWrap`'s own default. If a `cargo test` snapshot later shows unexpected wrapping, revisit.

- [ ] **Step 2: Build — expect breakage**

Run: `cargo build -p renderable`
Expected: FAIL — `MaxWidth`, `RowFill`, old `Margin::Chars`, `Layout::resolve_margin`, `Layout::available_width`, `page_bg_color` no longer exist. Note every error location.

- [ ] **Step 3: Fix `renderable`-internal call sites**

For each compile error inside `renderable`, migrate to the new API. The old `Layout` was used by the tree renderers and `attrs.rs` — those are addressed in Phase 2/3. For any other internal use, replace `Margin::Chars(n)` with `TargetValue::universal(Length::ch(n))` and drop `row_fill_strategy`/`page_bg_color` references.

- [ ] **Step 4: Run the layout module tests**

Run: `cargo test -p renderable layout`
Expected: PASS for all `layout::*` tests. `cargo build -p renderable` may still fail on tree-renderer code — that is expected and fixed in Phase 3.

- [ ] **Step 5: Commit**

```bash
git add renderable/src/layout/
git commit -m "feat(renderable): replace legacy Layout with the consolidated Layout primitive"
```

---

## Phase 2 — Tree Integration (`renderable`)

### Task 6: `NodeAttrs::layout` / `set_layout`; delete `LayoutHints`

**Files:**
- Modify: `renderable/src/tree/attrs.rs`
- Test: `renderable/src/tree/attrs.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the failing test** in `attrs.rs`'s test module:

```rust
#[test]
fn layout_roundtrips_through_node_attrs() {
    use crate::layout::{Alignment, Layout, Length, Margin};

    let layout = Layout {
        margin: Margin::x(Length::ch(2)),
        alignment: Alignment::Center,
        ..Layout::default()
    };
    let mut attrs = NodeAttrs::default();
    assert!(attrs.layout().is_none());
    attrs.set_layout(&layout);
    assert_eq!(attrs.layout(), Some(layout));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p renderable attrs::tests::layout_roundtrips_through_node_attrs`
Expected: FAIL — `set_layout` / `layout` not defined.

- [ ] **Step 3: Implement the accessors**

In `attrs.rs`, add to `impl NodeAttrs` (the `HintNamespace::LAYOUT` constant already exists):

```rust
/// Store a [`Layout`](crate::layout::Layout) on this node.
pub fn set_layout(&mut self, layout: &crate::layout::Layout) {
    if let Ok(value) = serde_json::to_value(layout) {
        self.set_hint(HintNamespace::LAYOUT, "layout", value);
    }
}

/// Read the [`Layout`](crate::layout::Layout) stored on this node, if any.
pub fn layout(&self) -> Option<crate::layout::Layout> {
    let value = self.get_hint(HintNamespace::LAYOUT, "layout")?;
    serde_json::from_value(value.clone()).ok()
}
```

- [ ] **Step 4: Delete `LayoutHints`**

Remove the `LayoutHints` struct and any `set_layout_hints`/`layout_hints` accessor methods from `attrs.rs`. Run `grep -rn "LayoutHints" renderable biscuit-terminal darkmatter` and note every reference for later tasks (the terminal renderer's `active_layout_hints` field is handled in Task 11).

- [ ] **Step 5: Run the test**

Run: `cargo test -p renderable attrs::tests::layout_roundtrips_through_node_attrs`
Expected: PASS. (`cargo build -p renderable` will still fail elsewhere — expected.)

- [ ] **Step 6: Commit**

```bash
git add renderable/src/tree/attrs.rs
git commit -m "feat(renderable): carry Layout on NodeAttrs, remove LayoutHints"
```

---

### Task 7: `TreeRenderable::tree_layout_hints` → `Option<Layout>`

**Files:**
- Modify: `renderable/src/tree/mod.rs`
- Test: `renderable/src/tree/mod.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the failing test** in `tree/mod.rs`'s test module:

```rust
#[test]
fn tree_renderable_can_supply_a_layout() {
    use crate::layout::{Layout, Length, Margin};

    struct Demo;
    impl TreeRenderable for Demo {
        fn render_tree(&self) -> RenderNode {
            RenderNode::paragraph(vec![RenderNode::text("hi")])
        }
        fn tree_layout(&self) -> Option<Layout> {
            Some(Layout {
                margin: Margin::x(Length::ch(1)),
                ..Layout::default()
            })
        }
    }
    assert!(Demo.tree_layout().is_some());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p renderable tree::tests::tree_renderable_can_supply_a_layout`
Expected: FAIL — `tree_layout` not defined.

- [ ] **Step 3: Retype the trait method**

In `tree/mod.rs`, replace the `tree_layout_hints` method on `TreeRenderable` with:

```rust
/// Optional layout for this component, seeded on its root node by the
/// tree renderers. Defaults to `None`.
fn tree_layout(&self) -> Option<crate::layout::Layout> {
    None
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p renderable tree::tests::tree_renderable_can_supply_a_layout`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/mod.rs
git commit -m "feat(renderable): retype TreeRenderable layout hook to Option<Layout>"
```

---

### Task 8: Block-only layout validation rule

**Files:**
- Modify: `renderable/src/tree/validate.rs`
- Test: `renderable/src/tree/validate.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the failing test** in `validate.rs`'s test module:

```rust
#[test]
fn layout_on_inline_node_is_a_validation_error() {
    use crate::layout::Layout;

    let mut text = RenderNode::text("hello");
    text.attrs.set_layout(&Layout::default());
    let root = RenderNode::root(vec![RenderNode::paragraph(vec![text])]);

    let report = validate(&root, ValidationMode::Full);
    assert!(
        report.has_errors(),
        "layout on an inline Text node must be an error"
    );
}

#[test]
fn layout_on_block_node_is_valid() {
    use crate::layout::Layout;

    let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
    para.attrs.set_layout(&Layout::default());
    let root = RenderNode::root(vec![para]);

    let report = validate(&root, ValidationMode::Full);
    assert!(!report.has_errors());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p renderable validate::tests::layout_on_inline_node_is_a_validation_error`
Expected: FAIL — no such rule yet.

- [ ] **Step 3: Implement the rule**

In `validate.rs`, in the per-node walk, after the existing block/inline classification, add: if `node.attrs.layout().is_some()` and the node's `NodeKind` is inline (`Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `InlineCode`, `Link`, `Image`, `FootnoteReference`, `SoftBreak`, `HardBreak`), push a `ValidationFinding` with `Severity::Error` and message `"layout attributes are permitted only on block-level nodes"`. Reuse the existing inline-kind predicate if `validate.rs` already has one; otherwise add a private `fn is_inline_kind(kind: &NodeKind) -> bool`.

- [ ] **Step 4: Run both tests**

Run: `cargo test -p renderable validate::tests`
Expected: PASS (both new tests plus existing).

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/validate.rs
git commit -m "feat(renderable): reject Layout on inline tree nodes during validation"
```

---

## Phase 3 — Renderers

### Task 9: Browser tree renderer lowers `Layout` to CSS

**Files:**
- Modify: `renderable/src/tree/render/browser.rs`
- Test: `renderable/src/tree/render/browser.rs` `#[cfg(test)]` (or the crate's browser render test file)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn browser_renderer_lowers_layout_to_css() {
    use crate::layout::{Alignment, Layout, Length, Margin, TargetValue};

    let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::x(Length::ch(2)),
        alignment: Alignment::Center,
        max_width: Some(TargetValue::universal(Length::Percent(80.0))),
        ..Layout::default()
    });
    let root = RenderNode::root(vec![para]);

    let rendered = render_browser_node(&root, &BrowserRenderOptions::default()).unwrap();
    let html = rendered.output.render();
    assert!(html.contains("margin-left:2ch") || html.contains("margin-left: 2ch"));
    assert!(html.contains("max-width:80%") || html.contains("max-width: 80%"));
    // Center + max_width => block centering via auto margins.
    assert!(html.contains("margin-left:auto") || html.contains("margin-right:auto")
        || html.contains("margin: 0 auto") );
}

#[test]
fn browser_renderer_lowers_vertical_margin_to_lh() {
    use crate::layout::{Layout, Length, Margin};

    let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::y(Length::ch(1)),
        ..Layout::default()
    });
    let root = RenderNode::root(vec![para]);
    let html = render_browser_node(&root, &BrowserRenderOptions::default())
        .unwrap()
        .output
        .render();
    assert!(html.contains("1lh"), "vertical Ch margin must lower to lh: {html}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p renderable browser_renderer_lowers_layout`
Expected: FAIL — layout not consulted.

- [ ] **Step 3: Implement layout lowering**

In `browser.rs`, add a helper that turns a `Layout` into an inline CSS declaration string and applies it to the fragment wrapper for the node being rendered:

```rust
/// Lower a `Layout` to inline CSS declarations for the browser target.
fn layout_to_css(layout: &crate::layout::Layout) -> String {
    use crate::layout::{Alignment, Length, TargetValue};
    use crate::target::RenderTarget;

    // Resolve a TargetValue<Length> for the Browser target.
    fn resolve(tv: &TargetValue<Length>) -> Option<&Length> {
        tv.resolve(RenderTarget::Browser)
    }
    // `vertical` selects lh vs ch for a `Ch` length.
    fn css_len(len: &Length, vertical: bool) -> String {
        match len {
            Length::Zero => "0".into(),
            Length::Ch(n) if vertical => format!("{n}lh"),
            Length::Ch(n) => format!("{n}ch"),
            Length::Percent(p) => format!("{p}%"),
            Length::Css(sizing) => sizing.to_string(),
        }
    }

    let m = &layout.margin;
    let mut decls: Vec<String> = Vec::new();
    if let Some(l) = resolve(&m.top) {
        decls.push(format!("margin-top:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&m.bottom) {
        decls.push(format!("margin-bottom:{}", css_len(l, true)));
    }
    if let Some(l) = resolve(&m.left) {
        decls.push(format!("margin-left:{}", css_len(l, false)));
    }
    if let Some(l) = resolve(&m.right) {
        decls.push(format!("margin-right:{}", css_len(l, false)));
    }
    if let Some(mw) = layout.max_width.as_ref().and_then(resolve) {
        decls.push(format!("max-width:{}", css_len(mw, false)));
        // Block alignment only meaningful with a width cap.
        match layout.alignment {
            Alignment::Center => {
                decls.push("margin-left:auto".into());
                decls.push("margin-right:auto".into());
            }
            Alignment::Right => {
                decls.push("margin-left:auto".into());
            }
            Alignment::Left => {}
        }
    }
    decls.join(";")
}
```

In the node-folding code, after building the fragment for a block node, if `node.attrs.layout()` is `Some(layout)` and `layout_to_css(&layout)` is non-empty, wrap the fragment's element with that inline `style` (use the existing `ComposableNode`/`BlockTag` styling path — wrap in a `<div style="...">` if the node has no own element). `word_wrap` lowering: `WordWrap::None` → append `white-space:nowrap`; any wrapping variant → append `overflow-wrap:break-word`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p renderable browser_renderer_lowers`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add renderable/src/tree/render/browser.rs
git commit -m "feat(renderable): lower Layout to CSS in the browser tree renderer"
```

---

### Task 10: Markdown tree renderer ignores `Layout`

**Files:**
- Modify: `renderable/src/tree/render/markdown.rs` (only if a test reveals leakage)
- Test: `renderable/src/tree/render/markdown.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the test**

```rust
#[test]
fn markdown_body_is_unchanged_when_layout_is_present() {
    use crate::layout::{Layout, Length, Margin};

    let plain = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::text("hi")])]);

    let mut para = RenderNode::paragraph(vec![RenderNode::text("hi")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::all(Length::ch(4)),
        ..Layout::default()
    });
    let with_layout = RenderNode::root(vec![para]);

    let opts = MarkdownRenderOptions::default();
    let a = render_markdown_node(&plain, &opts).unwrap();
    let b = render_markdown_node(&with_layout, &opts).unwrap();

    assert_eq!(a.output, b.output, "Markdown body must ignore Layout");
    assert!(
        b.diagnostics.is_empty(),
        "dropping layout from the Markdown body is by design — no diagnostics"
    );
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test -p renderable markdown_body_is_unchanged_when_layout_is_present`
Expected: PASS already (the Markdown renderer never reads layout). If it FAILS, the renderer is emitting something — remove that emission so the test passes. Either way, this test **locks** the behavior.

- [ ] **Step 3: Commit**

```bash
git add renderable/src/tree/render/markdown.rs
git commit -m "test(renderable): lock Markdown tree renderer to ignore Layout"
```

---

### Task 11: Terminal tree renderer applies `Layout`

**Files:**
- Modify: `biscuit-terminal/lib/src/render_tree/options.rs`
- Modify: `biscuit-terminal/lib/src/render_tree/render.rs`
- Test: `biscuit-terminal/lib/tests/` (new test file `tree_layout.rs`)

- [ ] **Step 1: Replace `active_layout_hints` with `Layout`**

In `render_tree/options.rs`, on `TerminalRenderContext`, replace the field `active_layout_hints: Option<LayoutHints>` with `active_layout: Option<renderable::layout::Layout>`, and rename the `with_layout` method to take `Option<renderable::layout::Layout>`. Update the import (`renderable::layout::Layout`) and remove the `LayoutHints` import.

- [ ] **Step 2: Write the failing test** — create `biscuit-terminal/lib/tests/tree_layout.rs`:

```rust
use biscuit_terminal::render_tree::{render_terminal_node, TerminalRenderOptions};
use renderable::layout::{Layout, Length, Margin};
use renderable::tree::RenderNode;

#[test]
fn terminal_renderer_applies_left_margin_in_cells() {
    let mut para = RenderNode::paragraph(vec![RenderNode::text("hello")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::x(Length::ch(4)),
        ..Layout::default()
    });
    let root = RenderNode::root(vec![para]);

    let opts = TerminalRenderOptions::default();
    let rendered = render_terminal_node(&root, &opts).unwrap();
    let first = rendered.output.lines().next().unwrap_or_default();
    let lead = first.len() - first.trim_start().len();
    assert!(lead >= 4, "expected >=4 leading cells, got {lead}: {first:?}");
}

#[test]
fn terminal_renderer_resolves_percent_margin_against_width() {
    // 10% of width 80 => 8 cells, round-half-up.
    let mut para = RenderNode::paragraph(vec![RenderNode::text("x")]);
    para.attrs.set_layout(&Layout {
        margin: Margin::x(Length::percent(10.0).unwrap()),
        ..Layout::default()
    });
    let root = RenderNode::root(vec![para]);

    let mut opts = TerminalRenderOptions::default();
    opts.context.width = 80;
    opts.context.available_width = 80;
    let rendered = render_terminal_node(&root, &opts).unwrap();
    let first = rendered.output.lines().next().unwrap_or_default();
    let lead = first.len() - first.trim_start().len();
    assert_eq!(lead, 8, "10% of 80 should resolve to 8 cells: {first:?}");
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p biscuit-terminal --test tree_layout`
Expected: FAIL — layout not applied.

- [ ] **Step 4: Implement terminal layout application**

In `render_tree/render.rs`, add a resolver and apply it when folding a block node:

```rust
/// Resolve a `TargetValue<Length>` to whole terminal cells against `width`.
fn resolve_cells(
    tv: &renderable::layout::TargetValue<renderable::layout::Length>,
    width: u32,
) -> u32 {
    use renderable::layout::Length;
    use renderable::target::RenderTarget;
    match tv.resolve(RenderTarget::Terminal) {
        Some(Length::Zero) | None => 0,
        Some(Length::Ch(n)) => *n,
        Some(Length::Percent(p)) => ((width as f32) * p / 100.0).round() as u32,
        // Css is invalid for the Terminal target in a universal value and
        // unreachable for a validated tree; treat as zero defensively.
        Some(Length::Css(_)) => 0,
    }
}
```

When the `Writer` folds a block node whose `attrs.layout()` is `Some(layout)`:
1. resolve `layout.margin.left`/`.right` to cells against the current `available_width`;
2. reduce the child render width by left+right (saturating);
3. render the node's content at the reduced width;
4. prefix each produced line with `left` spaces; alignment offset (when content is narrower than available width) follows `layout.alignment`;
5. emit `top`/`bottom` margin as that many blank lines.

This is the same arithmetic as `DarkmatterPage::apply_row_decoration` — reuse `biscuit_terminal::utils::block_constraint::visible_width` for ANSI-aware width. Vertical margin uses `Length::Ch(n)` → `n` blank rows.

- [ ] **Step 5: Run tests**

Run: `cargo test -p biscuit-terminal --test tree_layout`
Expected: PASS (both tests).

- [ ] **Step 6: Commit**

```bash
git add biscuit-terminal/lib/src/render_tree/ biscuit-terminal/lib/tests/tree_layout.rs
git commit -m "feat(biscuit-terminal): apply Layout in the terminal tree renderer"
```

---

## Phase 4 — `biscuit-terminal` Extension Migration

### Task 12: Adapt `LayoutTerminalExt` and bespoke call sites

**Files:**
- Modify: `biscuit-terminal/lib/src/utils/layout.rs`
- Modify: `biscuit-terminal/lib/src/prelude.rs`
- Modify: the bespoke call sites that reference removed types.
- Test: `biscuit-terminal/lib/src/utils/layout.rs` `#[cfg(test)]`

- [ ] **Step 1: Inventory the breakage**

Run: `cargo build -p biscuit-terminal 2>&1 | tee /tmp/bt-errors.txt`
The errors come from the re-export `pub use renderable::layout::{Alignment, Layout, Margin, MaxWidth, RowFill}` (`MaxWidth`/`RowFill` gone; `Margin` reshaped) and the 15 `apply_layout`/`apply_block_layout` call sites listed in the spec exploration.

- [ ] **Step 2: Update the re-exports**

In `utils/layout.rs` line ~14, change to `pub use renderable::layout::{Alignment, Layout, Length, Margin, TargetValue};`. In `prelude.rs`, change the layout re-export line to `pub use crate::utils::layout::{Alignment, Layout, LayoutTerminalExt, Length, Margin, TargetValue};` (drop `RowFill`).

- [ ] **Step 3: Rewrite `apply_layout` / `apply_block_layout`**

In `utils/layout.rs`, update both `LayoutTerminalExt` methods to read the new `Layout`:
- left/right margin: `resolve_cells(&layout.margin.left, terminal_width)` (copy the `resolve_cells` helper from Task 11 Step 4, or move it into `utils/layout.rs` and re-use it from the renderer).
- alignment: unchanged enum.
- drop all `row_fill_strategy` / `page_bg_color` branches — row fill / background are no longer part of `Layout` (Spec B). The methods keep their signatures (`fn apply_layout(&self, content: &str, terminal_width: u32) -> String`).
Update the doc-test in the trait doc comment to construct the new `Layout` (`Layout { alignment: Alignment::Left, ..Layout::default() }` still compiles; no change needed there).

- [ ] **Step 4: Fix the bespoke component call sites**

For each file below, the components build a `Layout` or `Margin` directly. Replace `Margin::Chars(n)` → `TargetValue::universal(Length::ch(n))`, `Margin::None` → `TargetValue::universal(Length::Zero)`, `Margin::Percent(p)` → `TargetValue::universal(Length::percent(p).expect("static percent"))`, and delete any `row_fill_strategy` / `page_bg_color` field initializers. Files (from the exploration): `text_block.rs`, `section.rs`, `list.rs`, `two_column.rs`, `status.rs`, `progress.rs`, `mermaid.rs`, `pad.rs`, `block_quote.rs`, `todo.rs`, `filesystem/mod.rs`, `graph_expression.rs`, `table/table.rs`, `prose/render.rs`, `terminal.rs`, `inline_content.rs`, `compose.rs`, plus CLI command files (`quote.rs`, `dir.rs`, `list.rs`, `columns.rs`, `prose.rs`, `shared.rs`) and `benches/rendering.rs`. Work the error list from Step 1 top to bottom; rebuild after each file.

- [ ] **Step 5: Update the `LayoutTerminalExt` unit tests**

In `utils/layout.rs`'s test module, replace any `Margin::Chars(_)` / `Layout { left_margin: ... }` construction with the new field shape (`Layout { margin: Margin::x(Length::ch(4)), ..Layout::default() }`). Keep one explicit test:

```rust
#[test]
fn apply_layout_indents_by_left_margin_cells() {
    let layout = Layout { margin: Margin::x(Length::ch(3)), ..Layout::default() };
    let out = layout.apply_layout("hi", 40);
    assert!(out.starts_with("   hi"));
}
```

- [ ] **Step 6: Build and test**

Run: `cargo test -p biscuit-terminal`
Expected: PASS (the layout-related drift tests may still show entries — addressed in Task 15).

- [ ] **Step 7: Commit**

```bash
git add biscuit-terminal/
git commit -m "refactor(biscuit-terminal): adapt LayoutTerminalExt and call sites to the new Layout"
```

---

## Phase 5 — `darkmatter` Migration

### Task 13: `DarkmatterPage` maps page margins onto `Layout`; deprecation conversions

**Files:**
- Modify: `darkmatter/lib/src/layout/types.rs`
- Modify: `darkmatter/lib/src/layout/page.rs`
- Modify: `darkmatter/lib/src/layout/context.rs`
- Test: `darkmatter/lib/src/layout/types.rs` `#[cfg(test)]`

- [ ] **Step 1: Write the failing conversion test** in `types.rs`'s test module:

```rust
#[test]
fn page_margin_converts_to_layout_margin() {
    use renderable::layout::{Length, Margin as RMargin, TargetValue};

    let page = PageMargin::all(2);
    let rendered: RMargin = page.into();
    assert_eq!(rendered.left, TargetValue::universal(Length::ch(2)));
    assert_eq!(rendered.top, TargetValue::universal(Length::ch(2)));
}

#[test]
fn page_alignment_converts_to_layout_alignment() {
    use renderable::layout::Alignment;
    assert_eq!(Alignment::from(PageAlignment::Center), Alignment::Center);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p darkmatter layout::types::tests::page_margin_converts`
Expected: FAIL — no `From` impl.

- [ ] **Step 3: Add deprecation conversions**

In `types.rs`, keep `PageMargin`, `PagePadding`, `PageFill`, `PageAlignment` (they are `pub` and used by the darkmatter CLI), mark each `#[deprecated(since = "...", note = "use renderable::layout::Layout")]`, and add conversions:

```rust
impl From<PageMargin> for renderable::layout::Margin {
    fn from(m: PageMargin) -> renderable::layout::Margin {
        use renderable::layout::{Length, TargetValue};
        let cell = |n: u16| TargetValue::universal(Length::ch(u32::from(n)));
        renderable::layout::Margin {
            top: cell(m.top),
            right: cell(m.right),
            bottom: cell(m.bottom),
            left: cell(m.left),
        }
    }
}

impl From<PageAlignment> for renderable::layout::Alignment {
    fn from(a: PageAlignment) -> renderable::layout::Alignment {
        match a {
            PageAlignment::Left => renderable::layout::Alignment::Left,
            PageAlignment::Center => renderable::layout::Alignment::Center,
            PageAlignment::Right => renderable::layout::Alignment::Right,
        }
    }
}
```

Add a `TryFrom<PageFill> for Option<TargetValue<Length>>` that maps `PageFill::Max(WidthUnit::Fixed(n))` / `Explicit(..)` to `Some(TargetValue::universal(Length::ch(n.into())))`, `PageFill::Full` to `None`, and `Pad`/`Indent` to a margin contribution (documented as handled by the caller). Percent `WidthUnit` maps to `Length::percent(..)?`.

- [ ] **Step 4: Rework `DarkmatterPage` rendering to seed a root `Layout`**

In `page.rs`, `DarkmatterPage` keeps its public builder API. Internally, in `render` / `render_to_browser`, instead of `apply_row_decoration` / `wrap_browser_html` doing bespoke margin math, build a `renderable::layout::Layout` from `self.margin`/`self.max_width`/`self.alignments` and apply it to the document root node before rendering through the tree renderers. Where the tree migration is not yet complete for a path, the existing bespoke decoration may remain temporarily — but the page margin must be expressed as a `Layout`, not `PageMargin` arithmetic. `context.rs` `LayoutContext` keeps working; its internals now consume the conversions from Step 3.

- [ ] **Step 5: Update the darkmatter CLI**

The CLI (`cli/src/args.rs`, `cli/src/output.rs`) constructs `DarkmatterPage` via its builder — that API is unchanged, so the CLI compiles as-is. Confirm with `cargo build -p darkmatter-cli` (or the CLI package name from `cargo metadata`). Fix any direct `PageFill`/`PageMargin` field access flagged by the `#[deprecated]` warnings by routing through the new conversions; `#[deprecated]` is a warning, not an error, so the CLI still builds.

- [ ] **Step 6: Build and test**

Run: `cargo test -p darkmatter` then `cargo build -p darkmatter` and the CLI package.
Expected: PASS; deprecation warnings are acceptable.

- [ ] **Step 7: Commit**

```bash
git add darkmatter/
git commit -m "refactor(darkmatter): map DarkmatterPage margins onto Layout with deprecation conversions"
```

---

## Phase 6 — Component Migration and Drift Burn-Down

### Task 14: The seven components emit `Layout`

**Files:**
- Modify: `biscuit-terminal/lib/src/components/section.rs`
- Modify: `biscuit-terminal/lib/src/components/list.rs`
- Modify: `biscuit-terminal/lib/src/components/progress.rs`
- Modify: `biscuit-terminal/lib/src/components/two_column.rs`
- Modify: `biscuit-terminal/lib/src/components/table/table.rs`
- Modify: `darkmatter/lib/src/markdown/yaml_block.rs`
- Test: per component, in its existing test module.

- [ ] **Step 1: `Table` — replace `LayoutHints` use with `Layout`**

`Table::render_tree_node` (table.rs ~line 1395) currently has a `margins_non_default` branch that intended to set `LayoutHints`. Replace it: when `self.layout` (a terminal `Layout`) has non-default margins, call `node.attrs.set_layout(&self.layout)` on the table `RenderNode`. Add a test:

```rust
#[test]
fn table_render_tree_node_carries_layout_when_margins_set() {
    use renderable::layout::{Length, Margin};
    let mut table = /* construct a minimal Table */;
    table.layout.margin = Margin::x(Length::ch(2));
    let node = table.render_tree_node().unwrap();
    assert!(node.attrs.layout().is_some());
}
```

- [ ] **Step 2: `Section`, `OrderedList`, `UnorderedList`, `Progress`, `TwoColumn`**

For each, if the component has a `layout` field with non-default margins/alignment/max-width, set it on the produced root `RenderNode` via `node.attrs.set_layout(&layout)`. Components with no layout field need no change here — they inherit `Layout::default()`. Verify each `render_tree_node` still compiles against the new `RenderNode`/`NodeAttrs` API (the `set_list_hints`/`set_progress_hints`/`set_columns_hints` calls are unaffected — those are separate hint structs, not `LayoutHints`).

- [ ] **Step 3: `YamlBlock`**

`YamlBlock::render_tree_node` (yaml_block.rs ~line 248) sets `CodeRenderHints` only — unaffected by the `LayoutHints` removal. Confirm it compiles; if `YamlBlock` carries a layout, set it via `set_layout`.

- [ ] **Step 4: Run component tests**

Run: `cargo test -p biscuit-terminal components::` and `cargo test -p darkmatter yaml_block`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add biscuit-terminal/lib/src/components/ darkmatter/lib/src/markdown/yaml_block.rs
git commit -m "feat(renderable): seven tree components emit the consolidated Layout"
```

---

### Task 15: Drift burn-down and full verification

**Files:**
- Modify: drift ledger entries as drift is resolved (driven by the comparison tests).

- [ ] **Step 1: Regenerate the drift report**

Run:
```bash
cd renderable && just drift-report
```
Record the new counts. Layout-related entries for `Section`, `Progress`, `Table`, `TwoColumn`, `UnorderedList`, `BlockQuote`, `YamlBlock` should be candidates to clear now that the tree renderers apply `Layout`.

- [ ] **Step 2: Resolve layout-attributable drift**

For each remaining drift entry whose cause is margin/alignment/max-width/wrapping, fix the renderer or component so tree output matches the bespoke renderer, then re-run the comparison test for that crate:
```bash
cargo test -p biscuit-terminal --test render_comparison
cargo test -p darkmatter --test render_comparison
```
Non-layout drift (color, code highlighting, etc.) is out of scope for Spec A — leave those entries.

- [ ] **Step 3: Full workspace verification**

Run:
```bash
cargo test -p renderable
cargo test -p biscuit-terminal
cargo test -p darkmatter
cargo clippy -p renderable -p biscuit-terminal -p darkmatter -- -D warnings
```
Expected: all PASS; clippy clean (deprecation warnings on the darkmatter page types are expected and allowed — scope them with `#[allow(deprecated)]` at the darkmatter-internal call sites if `-D warnings` rejects them).

- [ ] **Step 4: Update docs**

Update `.claude/skills/renderable/layout.md` and `.claude/skills/renderable/tree.md` to describe the new `Layout` / `TargetValue` / `Length` model and the removal of `LayoutHints`. Update `renderable`'s README if it documents layout.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "test(renderable): burn down layout drift and update layout docs"
```

---

## Self-Review

**Spec coverage:**
- D1/D2 (two primitives, single tier) — Tasks 2–5 define `Layout` with no semantic tier; appearance fields removed.
- D3 (`Length`, `TargetValue`, universal units, fallback) — Tasks 1–3.
- D4 (`Layout` struct, `padding` excluded, `word_wrap` kept) — Task 5.
- D5 (tree-resident, block-only validation) — Tasks 6–8.
- D6 (non-inherited composition, available width flows down) — Task 11 Step 4 (child width reduced by parent margins).
- D7 (per-target consumption) — Tasks 9 (browser), 10 (markdown), 11 (terminal).
- D8 (serde) — round-trip tests in Tasks 2, 5; documented JSON shape in the `Layout` rustdoc.
- D9 (migration, biscuit-terminal ergonomics, darkmatter compatibility) — Tasks 12, 13.
- Success criteria / required tests — Tasks 9–15 cover margins on all seven components, alignment ±max_width, percent at widths, per-target browser-only value, invalid universal units, block-only validation, composition, Markdown-unchanged, serde round-trip.

**Open items intentionally deferred to Spec B:** `style` frontmatter schema, `Style` slot system, appearance properties — not in this plan.

**Type consistency:** `Length`, `TargetValue<T>`, `Margin`, `Alignment`, `Layout`, `LayoutError`, `NodeAttrs::layout`/`set_layout`, `TreeRenderable::tree_layout`, `resolve_cells` — names are used identically across Tasks 1–15.

# Style Vocabulary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `renderable`'s `Layout`/`Style` types to the CSS box model — add `padding` + a `width` mode (incl. `fit-content`) to `Layout`, make `background` the only paint, delete `Fill`, and rename `Margin → Edges` — keeping the whole workspace compiling.

**Architecture:** A coordinated change to the `renderable` crate's core vocabulary plus the minimal consumer compile-fixes needed to keep `biscuit-terminal`, `darkmatter`, and the rest of the workspace building. New *behavior* (Width, padding, the adaptive-tint `Background` helper, defaulting, serde back-compat, `Style.fill` removal) is built test-first. Mechanical propagation (the `Margin → Edges` rename, the new `Layout` fields) is **compiler-driven**: change the type, then fix exactly the sites the compiler flags — which naturally excludes unrelated `Margin` types (e.g. `ratatui::Margin`) that never referenced renderable's.

**Tech Stack:** Rust 2024 edition, `serde` (+ `serde_json` for round-trip tests), the monorepo's `cargo`/`just` tooling, `md hash` (darkmatter) for skill-file hashes.

**Scope note:** This is one of four chapters in the [CSS Box Architecture](../2026-06-04-css-box-architecture/spec.md). It deliberately does **not** finish renderer behavior (terminal/browser painting the padding box, honoring `fit-content`) — that is *renderer-folds*. Here, removing `Fill` means the terminal renderer **temporarily loses per-component fill painting**; components that want the old tint set `Style.background = Some(Background::subtle()/pronounced())`, and exact band geometry is restored in *renderer-folds*. Per the architecture spec, characterization parity is a *reference*, not a contract.

**Spec:** [`spec.md`](spec.md). Read it before starting.

---

## File Structure

**Created:**
- `renderable/src/layout/width.rs` — the `Width` enum (`Auto | FitContent | Fixed`).
- `renderable/src/layout/edges.rs` — renamed from `margin.rs`; holds `Edges` (four-sided box) and `Alignment`.

**Modified (renderable — the vocabulary itself):**
- `renderable/src/layout/mod.rs` — module wiring, re-exports, `Layout` struct + `Default` + `validate`.
- `renderable/src/style.rs` — delete `Fill`/`FillBand`/`FillIntensity` + `Style.fill`; add the `Background` helper.
- `renderable/src/prelude.rs` — drop `Fill`/`FillBand`/`FillIntensity` exports, rename `Margin` export to `Edges`, add `Background`.

**Modified (consumers — compile-fixes):**
- `biscuit-terminal/lib/src/render_tree/style.rs` — remove fill lowering.
- `biscuit-terminal/lib/src/utils/layout.rs`, `biscuit-terminal/lib/src/prelude.rs` — re-export `Edges` not `Margin`.
- `biscuit-terminal/cli/src/commands/block.rs` — drop the `style.fill = …` write.
- Every other workspace site the compiler flags (renderable internals, darkmatter, biscuit-terminal components/tests, downstream CLIs).

**Modified (docs — part of the behavior change, not optional):**
- `.claude/skills/renderable/{layout.md,style.md,SKILL.md}`, `renderable/docs/layout-and-style.md`, any README mentioning `Fill`/`Margin`/"no padding".

**Verification baseline (run once before Task 1):**

- [ ] **Step 0: Confirm a green starting point**

Run: `cargo build -p renderable -p biscuit-terminal -p darkmatter`
Expected: builds clean (warnings from the untracked `darkmatter/lib/src/cli.rs` are pre-existing and unrelated).

Run: `cargo test -p renderable --no-run`
Expected: test binaries compile.

---

## Task 1: Add the `Width` enum (additive — nothing breaks)

**Files:**
- Create: `renderable/src/layout/width.rs`
- Modify: `renderable/src/layout/mod.rs` (module decl + re-export)

- [ ] **Step 1: Write the failing test**

Create `renderable/src/layout/width.rs` with only the tests first:

```rust
//! The `Width` content-box sizing mode for [`Layout`](super::Layout).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Length, TargetValue};

    #[test]
    fn default_is_auto() {
        assert_eq!(Width::default(), Width::Auto);
    }

    #[test]
    fn serde_tags_are_snake_case() {
        assert_eq!(serde_json::to_string(&Width::Auto).unwrap(), "\"auto\"");
        assert_eq!(
            serde_json::to_string(&Width::FitContent).unwrap(),
            "\"fit_content\""
        );
        let fixed = Width::Fixed(TargetValue::universal(Length::ch(60)));
        let json = serde_json::to_string(&fixed).unwrap();
        assert_eq!(json, r#"{"fixed":{"universal":{"ch":60}}}"#);
        assert_eq!(serde_json::from_str::<Width>(&json).unwrap(), fixed);
    }

    #[test]
    fn fit_content_constructor() {
        assert_eq!(Width::fit_content(), Width::FitContent);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p renderable layout::width -- --nocapture`
Expected: FAIL — `cannot find type Width in this scope` (module not declared / type not defined).

- [ ] **Step 3: Write minimal implementation**

Prepend to `renderable/src/layout/width.rs` (above the `#[cfg(test)]` block):

```rust
use serde::{Deserialize, Serialize};

use crate::layout::{Length, TargetValue};
use crate::layout::length::LayoutError;

/// How a block sizes its content box horizontally (CSS `width`).
///
/// Composes with [`Layout::max_width`](super::Layout::max_width): the cap is a
/// separate, orthogonal field, so `FitContent` + a `max_width` cap, or `Auto`
/// + a cap, are both expressible.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Width {
    /// Fill the parent's available width (CSS `width: auto` on a block).
    #[default]
    Auto,
    /// Size to the content's widest line (CSS `width: fit-content`).
    FitContent,
    /// An explicit width (cells / percent / per-target CSS length).
    Fixed(TargetValue<Length>),
}

impl Width {
    /// The content-hugging width mode.
    pub fn fit_content() -> Width {
        Width::FitContent
    }

    /// Validate the contained length, if any.
    ///
    /// ## Errors
    /// Propagates the first [`LayoutError`] from a `Fixed` value's
    /// [`TargetValue::validate`].
    pub fn validate(&self) -> Result<(), LayoutError> {
        match self {
            Width::Auto | Width::FitContent => Ok(()),
            Width::Fixed(value) => value.validate(),
        }
    }
}
```

Add to `renderable/src/layout/mod.rs` (with the other `mod` decls and `pub use`):

```rust
mod width;
// ... existing ...
pub use width::Width;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p renderable layout::width`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add renderable/src/layout/width.rs renderable/src/layout/mod.rs
git commit -m "feat(renderable): add Width content-box sizing enum"
```

---

## Task 2: Rename `Margin` → `Edges` (compiler-driven sweep)

The type `renderable::layout::Margin` becomes `Edges`. The **field** `Layout.margin` keeps its name (serde stability). Unrelated `Margin` types in other crates (e.g. `ratatui::Margin`) are *not* touched — the compiler will only flag sites that referenced renderable's type.

**Files:**
- Rename: `renderable/src/layout/margin.rs` → `renderable/src/layout/edges.rs`
- Modify: `renderable/src/layout/mod.rs`, `renderable/src/prelude.rs`
- Modify: `biscuit-terminal/lib/src/utils/layout.rs`, `biscuit-terminal/lib/src/prelude.rs`
- Modify: every site `cargo build` flags (renderable internals, biscuit-terminal, darkmatter, downstream crates)

- [ ] **Step 1: Rename the module file and the type within it**

```bash
git mv renderable/src/layout/margin.rs renderable/src/layout/edges.rs
```

In `renderable/src/layout/edges.rs` replace the type name only (the four-sided box). Change the doc line, the struct, and the three constructor return types:

```rust
//! Edge box (margins / padding) and alignment for [`Layout`](super::Layout).

/// A four-sided edge box (used for both `margin` and `padding`). Each side is
/// a [`TargetValue<Length>`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edges {
    pub top: TargetValue<Length>,
    pub right: TargetValue<Length>,
    pub bottom: TargetValue<Length>,
    pub left: TargetValue<Length>,
}
```

Update `impl Default for Edges`, `impl Edges`, and the `-> Margin` return types of `all` / `x` / `y` to `-> Edges`. Update the in-file `#[cfg(test)]` references (`Margin::default()` → `Edges::default()`, `Margin::x(..)` → `Edges::x(..)`). `Alignment` in this file is unchanged.

- [ ] **Step 2: Update renderable module wiring + re-exports**

`renderable/src/layout/mod.rs`:

```rust
mod edges;     // was: mod margin;
// ...
pub use edges::{Alignment, Edges};   // was: pub use margin::{Alignment, Margin};
```

In the `Layout` struct keep the field name `margin` but change its **type**:

```rust
    pub margin: Edges,   // field name unchanged; type Margin -> Edges
```

and in `impl Default for Layout`: `margin: Edges::default(),`.

`renderable/src/prelude.rs`: change the `Margin` export to `Edges` (leave everything else; `Fill` handled in Task 4).

- [ ] **Step 3: Update biscuit-terminal's re-export so downstream still resolves**

`biscuit-terminal/lib/src/utils/layout.rs:15` and `biscuit-terminal/lib/src/prelude.rs:48`: replace `Margin` with `Edges` in the `pub use renderable::layout::{...}` / re-export lists.

- [ ] **Step 4: Build the workspace and let the compiler enumerate the rest**

Run: `cargo build --workspace 2>&1 | rg "cannot find|expected.*Margin|Margin" | head -50`
Expected: a list of `error[E0412]: cannot find type \`Margin\`` (and similar) at exactly the sites that used renderable's type.

- [ ] **Step 5: Fix each flagged site**

For every compiler error, replace the renderable `Margin` type reference with `Edges`:
- imports: `use renderable::layout::{… Margin …}` → `… Edges …`; `use biscuit_terminal::utils::layout::Margin` → `… Edges`.
- type annotations / constructors / literals: `Margin::all/x/y/default` → `Edges::…`; `Margin { … }` → `Edges { … }`.

Do **not** change `PageMargin` (a different darkmatter type) and do **not** change `Margin` references the compiler did *not* flag (those resolve to a different crate's `Margin`, e.g. `ratatui::Margin`). Re-run the build after each batch:

Run: `cargo build --workspace`
Expected (when done): builds clean.

- [ ] **Step 6: Verify the rename is complete and correct**

Run: `rg -n 'renderable::layout::Margin|layout::Margin\b' --type rust`
Expected: no matches.

Run: `cargo build --workspace && cargo test --workspace --no-run`
Expected: builds + all test binaries compile.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor(renderable): rename layout Margin type to Edges"
```

---

## Task 3: Add `padding` + `width` to `Layout` (with serde back-compat)

**Files:**
- Modify: `renderable/src/layout/mod.rs` (`Layout` struct, `Default`, `validate`, tests)
- Modify: any `Layout { … }` literal without a `..default` spread (the compiler will flag them)

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` in `renderable/src/layout/mod.rs`:

```rust
    #[test]
    fn default_layout_has_zero_padding_and_auto_width() {
        let layout = Layout::default();
        assert_eq!(layout.padding, Edges::default());
        assert_eq!(layout.width, Width::Auto);
    }

    #[test]
    fn old_payload_without_padding_or_width_deserializes_with_defaults() {
        // A render tree serialized before `padding`/`width` existed.
        let json = r#"{
            "margin": {
                "top": { "universal": "zero" },
                "right": { "universal": "zero" },
                "bottom": { "universal": "zero" },
                "left": { "universal": { "ch": 2 } }
            },
            "alignment": "left",
            "max_width": null,
            "word_wrap": "none"
        }"#;
        let layout: Layout = serde_json::from_str(json).unwrap();
        assert_eq!(layout.padding, Edges::default());
        assert_eq!(layout.width, Width::Auto);
        assert_eq!(layout.margin.left, TargetValue::universal(Length::ch(2)));
    }

    #[test]
    fn validate_rejects_bad_padding_percent() {
        let layout = Layout {
            padding: Edges {
                left: TargetValue::universal(Length::Percent(150.0)),
                ..Edges::default()
            },
            ..Layout::default()
        };
        assert!(layout.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_fixed_width_percent() {
        let layout = Layout {
            width: Width::Fixed(TargetValue::universal(Length::Percent(150.0))),
            ..Layout::default()
        };
        assert!(layout.validate().is_err());
    }

    #[test]
    fn layout_with_padding_and_width_serde_roundtrips() {
        let layout = Layout {
            padding: Edges::x(Length::ch(4)),
            width: Width::Fixed(TargetValue::universal(Length::ch(60))),
            ..Layout::default()
        };
        let json = serde_json::to_string(&layout).unwrap();
        let back: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(layout, back);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p renderable layout::tests -- --nocapture`
Expected: FAIL — `no field \`padding\` on type \`Layout\`` / `no field \`width\``.

- [ ] **Step 3: Add the fields, defaults, and validation**

In `renderable/src/layout/mod.rs`, add the imports `use crate::layout::Width;` if not already re-exported in scope, then extend `Layout`:

```rust
pub struct Layout {
    pub margin: Edges,
    /// Reserved inner space, painted by `Style.background`. `#[serde(default)]`
    /// so trees serialized before `padding` existed deserialize to zero.
    #[serde(default)]
    pub padding: Edges,
    /// Content-box width mode. `#[serde(default)]` → `Width::Auto`.
    #[serde(default)]
    pub width: Width,
    pub max_width: Option<TargetValue<Length>>,
    pub alignment: Alignment,
    pub word_wrap: WordWrap,
}
```

Extend `impl Default for Layout`:

```rust
        Self {
            margin: Edges::default(),
            padding: Edges::default(),
            width: Width::Auto,
            max_width: None,
            alignment: Alignment::default(),
            word_wrap: WordWrap::None,
        }
```

Extend `Layout::validate`:

```rust
    pub fn validate(&self) -> Result<(), LayoutError> {
        self.margin.validate()?;
        self.padding.validate()?;
        self.width.validate()?;
        if let Some(max_width) = &self.max_width {
            max_width.validate()?;
        }
        Ok(())
    }
```

(`Edges::validate` already exists from the renamed `margin.rs`; it validates all four sides.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p renderable layout::tests`
Expected: PASS (new + existing layout tests).

- [ ] **Step 5: Fix any `Layout { … }` literal the field-add broke**

Run: `cargo build --workspace 2>&1 | rg "missing field|cannot find" | head -30`
Expected: at minimum `renderable/src/tree/render/browser.rs:~3802` — a `Layout { margin, alignment, max_width }` literal with no spread.

For each, add `..Layout::default()` (preferred) or the two new fields explicitly. Example for `browser.rs`:

```rust
let layout = Layout {
    margin: Edges::x(Length::ch(2)),
    alignment: Alignment::Center,
    max_width: Some(TargetValue::universal(Length::ch(40))),
    ..Layout::default()
};
```

Re-run until clean:

Run: `cargo build --workspace && cargo test --workspace --no-run`
Expected: builds + test binaries compile.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(renderable): add padding and width to Layout with serde defaults"
```

---

## Task 4: Delete `Fill`; add the `Background` adaptive-tint helper

**Files:**
- Modify: `renderable/src/style.rs` (delete `Fill`/`FillBand`/`FillIntensity` + `Style.fill`; add `Background`; update `inherited_from`, doc/json examples, tests)
- Modify: `renderable/src/prelude.rs` (drop fill exports, add `Background`)
- Modify: `biscuit-terminal/lib/src/render_tree/style.rs` (remove fill lowering)
- Modify: `biscuit-terminal/cli/src/commands/block.rs` (drop `style.fill = …`)
- Modify: any other site the compiler flags (`biscuit-terminal/lib/src/render_tree/render.rs`, etc.)

- [ ] **Step 1: Write the failing tests for the `Background` helper + `is_empty`**

In `renderable/src/style.rs` tests, add:

```rust
    #[test]
    fn background_subtle_matches_former_fill_tints() {
        use crate::color::{BasicColor, Color, ColorMode, RgbColor};

        let bg = Background::subtle();
        let per_mode = bg.resolve(RenderTarget::Terminal).unwrap();
        assert_eq!(
            per_mode.resolve(ColorMode::Dark),
            &Color::Rgb(RgbColor::new(30, 30, 34, BasicColor::Black))
        );
        assert_eq!(
            per_mode.resolve(ColorMode::Light),
            &Color::Rgb(RgbColor::new(235, 235, 238, BasicColor::White))
        );
    }

    #[test]
    fn background_pronounced_matches_former_fill_tints() {
        use crate::color::{BasicColor, Color, ColorMode, RgbColor};

        let per_mode = Background::pronounced().resolve(RenderTarget::Terminal).unwrap().clone();
        assert_eq!(
            per_mode.resolve(ColorMode::Dark),
            &Color::Rgb(RgbColor::new(50, 50, 56, BasicColor::Black))
        );
        assert_eq!(
            per_mode.resolve(ColorMode::Light),
            &Color::Rgb(RgbColor::new(215, 215, 220, BasicColor::White))
        );
    }

    #[test]
    fn style_with_only_background_is_not_empty() {
        let style = Style {
            background: Some(Background::subtle()),
            ..Style::default()
        };
        assert!(!style.is_empty());
    }

    #[test]
    fn old_style_payload_with_fill_key_still_deserializes() {
        // serde ignores unknown fields by default, so a pre-deletion tree that
        // still carries "fill": null deserializes fine.
        let json = r#"{
            "color": null, "background": null,
            "emphasis": { "bold": false, "dim": false, "italic": false,
                          "strikethrough": false, "blink": false, "underline": null },
            "border": null, "fill": null
        }"#;
        let style: Style = serde_json::from_str(json).unwrap();
        assert!(style.is_empty());
    }
```

Note the test module needs `use renderable::target::RenderTarget;` — add `use crate::target::RenderTarget;` to the `mod tests` imports.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p renderable style:: -- --nocapture`
Expected: FAIL — `cannot find … Background` / `no field \`fill\`` is still present.

- [ ] **Step 3: Delete the fill types and the `fill` field; add `Background`**

In `renderable/src/style.rs`:

1. Delete the `FillIntensity` enum, the `FillBand` enum, and the `Fill` struct (the three `#[derive(...)] pub enum/struct` blocks).
2. In `struct Style`, delete the `pub fill: Option<Fill>,` field and its doc comment.
3. In `Style::inherited_from`, delete the `fill: self.fill.clone(),` line.
4. In the `Style` doc comment + the `## Serialized shape` JSON example, remove `fill` (and the sentence calling `fill` a box-painting field — keep `background`, `border`).
5. Delete the `fill_serde_roundtrip` test and remove `fill: Some(Fill::default())` / `assert!(merged.fill.is_none());` lines from `style_inheritance_box_painting_does_not_inherit` and `fill: None,` from `style_serde_roundtrip`.
6. Add the `Background` helper near `Style`:

```rust
/// Constructors for the adaptive background tints that the deleted `Fill`
/// intensities used to supply implicitly.
///
/// These return the `TargetValue<PerMode<Color>>` value `Style.background`
/// holds; `Background` itself is a zero-sized constructor namespace, never
/// stored, so serialized `Style.background` still contains only the color
/// value.
pub struct Background;

impl Background {
    /// A faint adaptive tint (former `FillIntensity::Subtle`):
    /// `rgb(235,235,238)` on light, `rgb(30,30,34)` on dark.
    pub fn subtle() -> TargetValue<PerMode<Color>> {
        use crate::color::{BasicColor, RgbColor};
        TargetValue::universal(PerMode::adaptive(
            Color::Rgb(RgbColor::new(235, 235, 238, BasicColor::White)),
            Color::Rgb(RgbColor::new(30, 30, 34, BasicColor::Black)),
        ))
    }

    /// A strong adaptive tint (former `FillIntensity::Pronounced`):
    /// `rgb(215,215,220)` on light, `rgb(50,50,56)` on dark.
    pub fn pronounced() -> TargetValue<PerMode<Color>> {
        use crate::color::{BasicColor, RgbColor};
        TargetValue::universal(PerMode::adaptive(
            Color::Rgb(RgbColor::new(215, 215, 220, BasicColor::White)),
            Color::Rgb(RgbColor::new(50, 50, 56, BasicColor::Black)),
        ))
    }
}
```

In `renderable/src/prelude.rs`: remove `Fill, FillBand, FillIntensity` from the `pub use crate::style::{…}` list and add `Background`.

- [ ] **Step 4: Run the renderable tests**

Run: `cargo test -p renderable style::`
Expected: PASS (new Background/is_empty tests + the trimmed existing ones).

- [ ] **Step 5: Remove fill lowering from the terminal renderer**

In `biscuit-terminal/lib/src/render_tree/style.rs`:
- Drop `Fill, FillBand, FillIntensity` from the `use renderable::style::{…}` import (line ~23) and the module doc bullet referencing `Fill` (lines ~13-15).
- In `paint_text`, delete the `let fill = style.fill…` binding and the `band_offset`/`band_width` logic that depends on it; the background now comes solely from `style.background`. Concretely, replace the band-aware block so painting uses only the explicit background SGR, and each line is emitted without fill offset/padding:

```rust
    let mut open = emphasis_sgr(style, term);
    if let Some(fg) = style.color.as_ref().and_then(|c| resolve_color(c, mode)) {
        open.push_str(&color_sgr(fg, depth, false).unwrap_or_default());
    }
    if let Some(bg) = style
        .background
        .as_ref()
        .and_then(|c| resolve_color(c, mode))
        .and_then(|c| color_sgr(c, depth, true))
    {
        open.push_str(&bg);
    }

    if open.is_empty() {
        return content.to_string();
    }
    let mut out = String::new();
    for (idx, line) in content.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        if line.is_empty() {
            continue;
        }
        out.push_str(&open);
        out.push_str(line);
        out.push_str(SGR_RESET);
    }
    out
```

- Delete the now-unused helpers `fill_band`, `fill_sgr`, `resolve_inset_columns`, the `DEFAULT_INDENT` const, and the `available_width` parameter threading if it becomes unused (let the compiler guide which become dead). Delete the fill-specific tests (`fill_paints_a_background_band`, `fill_band_padded_paints_only_the_content_width`, `implicit_fill_tint_degrades_with_color_depth`, and any other `fill_*`).

> Behavior note: per-component fill *band geometry* is intentionally not reproduced here; it returns via `padding` + `background` in *renderer-folds*. The adaptive tint itself is preserved through `Background::subtle()/pronounced()` set on `style.background`.

- [ ] **Step 6: Remove the CLI fill write and fix remaining flagged sites**

`biscuit-terminal/cli/src/commands/block.rs`: delete the `use renderable::style::Fill;` and the `style.fill = Some(Fill { … })` block (and the `--fill` arg plumbing that fed it, if it is now dead — follow the compiler).

Run: `cargo build --workspace 2>&1 | rg "FillBand|FillIntensity|\bFill\b|no field .fill|cannot find" | head -40`
Fix each flagged site (e.g. `biscuit-terminal/lib/src/render_tree/render.rs:~3458,3472`). Re-run until clean:

Run: `cargo build --workspace`
Expected: builds clean.

- [ ] **Step 7: Verify the deletion and run tests**

Run: `rg -n 'FillBand|FillIntensity' renderable/ biscuit-terminal/ darkmatter/ --type rust`
Expected: no matches (outside this `features/` plan/spec text).

Run: `cargo test -p renderable -p biscuit-terminal --no-run && cargo test -p renderable`
Expected: compiles; renderable tests pass.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(renderable): delete Fill; add Background adaptive-tint helper"
```

---

## Task 5: Update skills and docs (part of the behavior change)

**Files:**
- Modify: `.claude/skills/renderable/layout.md`, `.claude/skills/renderable/style.md`, `.claude/skills/renderable/SKILL.md`
- Modify: `renderable/docs/layout-and-style.md`
- Modify: any README mentioning `Fill`, `Margin`, or "no padding" (find with the command below)

- [ ] **Step 1: Find every doc reference**

Run: `rg -n 'Fill\b|FillBand|FillIntensity|renderable::layout::Margin|no padding|has no padding' .claude/skills/renderable renderable/docs renderable/README.md biscuit-terminal/README.md`
Expected: a list of doc lines to fix.

- [ ] **Step 2: Rewrite the references**

Apply these content changes:
- `layout.md` / `layout-and-style.md`: rename `Margin` → `Edges`; state `Layout` now has `padding` and a `width` mode (`Auto|FitContent|Fixed`) plus `max_width`; remove any "Layout deliberately has no padding" claim; document the `#[serde(default)]` back-compat for `padding`/`width` and the `Width` snake_case tags.
- `style.md` / `layout-and-style.md`: delete the `Fill` / `FillBand` / `FillIntensity` section; state that `background` paints the content + padding box and that the former `Subtle`/`Pronounced` tints are now `Background::subtle()/pronounced()`.
- `SKILL.md`: fix the module table rows that name `Margin` / `Fill`.

- [ ] **Step 3: Regenerate skill-file hashes**

The repo convention hashes skill markdown via darkmatter. For each edited skill file under `.claude/skills/renderable/`:

Run: `md hash .claude/skills/renderable/layout.md` (and `style.md`, `SKILL.md`)
Then update each file's `hash:` frontmatter with the reported value (or use the tool's `--save` form if available).

Run: `rg -n 'no padding|FillBand|FillIntensity' .claude/skills/renderable renderable/docs`
Expected: no stale claims remain.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/renderable renderable/docs renderable/README.md biscuit-terminal/README.md
git commit -m "docs(renderable): document Edges, Layout padding/width, and Fill removal"
```

---

## Task 6: Whole-workspace verification

**Files:** none (verification only)

- [ ] **Step 1: Acceptance-criteria greps**

Run: `rg -n 'renderable::layout::Margin|layout::Margin\b' --type rust`
Expected: none (AC #2).

Run: `rg -n 'FillBand|FillIntensity' --type rust`
Expected: none (AC #3).

- [ ] **Step 2: Build + test the directly-affected crates**

Run: `cargo build --workspace`
Expected: clean.

Run: `cargo test -p renderable -p biscuit-terminal -p darkmatter`
Expected: PASS. Investigate any failure; remember that *fill band geometry* changes are expected references (not contracts) — a snapshot whose only diff is the now-absent per-component fill band is acceptable and should be re-baselined with a note, per the architecture spec. A *compile* failure or an unrelated regression is not acceptable.

- [ ] **Step 3: Final commit (if any snapshots were re-baselined)**

```bash
git add -A
git commit -m "test(renderable): re-baseline snapshots for Fill removal (band geometry deferred to renderer-folds)"
```

---

## Self-Review Notes (for the executor)

- **Spec AC coverage:** AC1 (Task 3), AC2 (Task 2 + Task 6 grep), AC3 (Task 4 + Task 6 grep), AC4 (Task 4 Background tests), AC5 (Task 3 default tests + the existing `style_default_is_empty`), AC6 (Task 1 + Task 3 serde/validate tests), AC7 (every task ends on `cargo build --workspace` green), AC8 (Task 5).
- **Order matters:** Task 2 (rename) precedes Task 3 (fields) so the new fields are typed `Edges`. Task 4 (Fill deletion) is last among the type changes because it has the widest consumer reach.
- **`Background` naming:** it is `renderable::style::Background` with associated `subtle()` / `pronounced()` returning `TargetValue<PerMode<Color>>` — used consistently in Task 4's tests and impl.
- **Do not** introduce a `type Margin = Edges` alias (spec "Open Questions": no alias).

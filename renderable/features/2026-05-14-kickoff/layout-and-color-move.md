# Layout + Color Move Spec

> **Note.** Per [`decisions.md`](./decisions.md) item 6, `Layout` is **not**
> consumed by `PageOptions` or browser page rendering — page styling goes
> through the page `Stylesheet`. `Layout` still moves to `renderable` as
> described here, as a future cross-target layout primitive. Separately,
> `decisions.md` item 10 renames the stylesheet declaration-block type
> `Stylesheet → CssStyle`; references to `Stylesheet` below predate that
> rename.

Move `Layout` and `Color` (and their satellite types) from `biscuit-terminal` to the new `renderable` library, so render-target traits (`TerminalRenderable`, `BrowserRenderable`, future `MarkdownRenderable`) can share a common style vocabulary without depending on the terminal crate.

This spec is sequenced **after** the Stylesheet move so `CssColor` is available as a portable cross-target color reference if needed.

## Goals

1. `renderable` becomes the single source of truth for layout + color **data**.
2. `biscuit-terminal` keeps all terminal-specific **rendering** behavior (ANSI emission, width math).
3. **Zero source breakage for downstream callers** — every existing `use biscuit_terminal::utils::{layout,color}::…` path resolves via re-export.
4. No new feature flags. No new crates beyond `renderable`.
5. No regressions in `biscuit-terminal` test or example output (byte-identical ANSI streams for fixed terminal inputs).

## Non-Goals

- No API additions beyond what's necessary for the move (don't grow `Layout` or `Color` while in transit).
- No changes to `Renderable` / `BrowserRenderable` trait *shape* in this spec — that belongs to kickoff §1 (`spec.md`). This spec just ensures their backing types live in the right place when those renames happen.
- Not extracting `wrap_lines` / `visible_width` / `split_lines` from `block_constraint.rs` — those stay in `biscuit-terminal`.
- Not extracting the `RenderableWrapper` trait — terminal-coupled, stays put.

## Prereqs

- The Stylesheet move (separate spec) is **landed** before this work begins. Rationale:
    - `CssColor` is the natural cross-target color form for stylesheet emission, and
    - if `Color` and `CssColor` end up in the same crate (`renderable`), we want one consistent home.

## Architectural Overview

The move follows the same data-vs-rendering split applied to Stylesheet:

```txt
                ┌─────────────────────────────────────────────────────┐
                │                     renderable                      │
                │                                                     │
                │   layout::{Layout, Margin, Alignment, RowFill,      │
                │            MaxWidth, WordWrap}                      │
                │   color::{Color, BasicColor, RgbColor, WebColor,    │
                │           Tailwind, HdrColor, Octet, OctetError}    │
                │   stylesheet::{Stylesheet, CssColor, …}             │
                │                                                     │
                │   (NO Terminal, NO ANSI emission, NO Prose)         │
                └────────────────────────┬────────────────────────────┘
                                         │ depended on by
                                         ▼
                ┌─────────────────────────────────────────────────────┐
                │                  biscuit-terminal                   │
                │                                                     │
                │   - TermColor trait + impls (ANSI emission)         │
                │   - LayoutTerminalExt (apply_layout, apply_block_…) │
                │   - RenderableWrapper trait + color wrappers        │
                │   - block_constraint (split_lines, visible_width,   │
                │     wrap_lines)                                     │
                │   - re-export shim:                                 │
                │       pub use renderable::{layout, color}::*        │
                └─────────────────────────────────────────────────────┘
```

### Why both, together

Color and Layout are entangled in one place: `Layout::page_bg_color: Option<Color>`. The three options previously considered (move Color too / generic param / swap for `CssColor`) are won by **moving Color** in this revision, because:

- Color is already mostly pure data — the entanglement with biscuit-terminal is narrow (single back-edge: `wrappers.rs`).
- Keeping `Color` as the layout field preserves the rich palette types callers already use (Tailwind, Web, RGB-with-fallback). `CssColor` would lose Tailwind/Web/BasicColor nuance.
- Sharing destination crate with `CssColor` opens future ergonomic conversions without cross-crate boundaries.

## Inventory

### Layout subsystem (`biscuit-terminal/lib/src/utils/layout.rs` — 871 lines)

| Symbol | Move to renderable | Notes |
|---|:-:|---|
| `Layout` struct (fields only) | ✅ | Pure data |
| `Alignment` enum | ✅ | Pure data |
| `Margin` enum + `Margin::add_chars` | ✅ | Numeric only |
| `RowFill` enum | ✅ | Pure data |
| `MaxWidth` enum | ✅ | Pure data |
| `Layout::resolve_margin` | ✅ | Pure arithmetic |
| `Layout::available_width` | ✅ | Pure arithmetic |
| `Layout::new`, `Layout::default` | ✅ | Pure |
| `WordWrap` (from `wrap_policy.rs`, 144 lines) | ✅ | Pure data; move policy enum, leave `wrap_lines` |
| `Layout::apply_layout` | ❌ | ANSI-width math; becomes `LayoutTerminalExt::apply_layout` |
| `Layout::apply_block_layout` | ❌ | Same — `LayoutTerminalExt::apply_block_layout` |
| `RenderableWrapper` trait | ❌ | Takes `&Terminal` |

### Color subsystem (`biscuit-terminal/lib/src/utils/color/` — 10 files, ~70 KB)

| File | Symbols | Move | Notes |
|---|---|:-:|---|
| `mod.rs` | module wiring + `TermColor` trait | partial | data exports move; `TermColor` trait stays |
| `color_enum.rs` | `Color`, `Color::to_rgb` | ✅ | Pure data |
| `basic.rs` | `BasicColor`, `FgBg`, `basic_color_to_rgb`, `color_code`, `impl TermColor for BasicColor` | partial | Enum + helpers move; `TermColor` impl stays |
| `rgb.rs` | `RgbColor`, `impl TermColor for RgbColor` | partial | Same pattern |
| `hdr.rs` | `HdrColor`, `impl TermColor for HdrColor` | partial | Same pattern |
| `web.rs` | `WebColor`, `WEB_COLOR_LOOKUP`, `impl TermColor for WebColor` | partial | Same pattern; 148 named colors |
| `tailwind.rs` | `Tailwind`, `Tailwind::to_hdr_color` | ✅ | Pure data |
| `octet.rs` | `Octet`, `OctetError` | ✅ | Pure data + thiserror |
| `wrappers.rs` | `BasicColorWrapper`, `RgbColorWrapper`, `TailwindColorWrapper`, `WebColorWrapper` (`impl RenderableWrapper`) | ❌ | Depend on `Terminal` / `RenderableWrapper`; stays |
| `tests.rs` | unit tests | split | Data-only tests move with their types; ANSI-emission tests stay |

### Internal dependency edges within `color/`

```txt
octet.rs        →   (no internal deps)
basic.rs        →   TermColor
rgb.rs          →   BasicColor, Octet, TermColor
hdr.rs          →   BasicColor, Octet, TermColor
web.rs          →   BasicColor, RgbColor, TermColor
tailwind.rs     →   BasicColor, HdrColor
color_enum.rs   →   BasicColor, RgbColor, Tailwind, WebColor,
                    basic_color_to_rgb, WEB_COLOR_LOOKUP
wrappers.rs     →   BasicColor, RgbColor, Tailwind, WebColor,
                    ⚠ Terminal, ⚠ RenderableWrapper   (back-edges)
```

The **only** back-edge from `color/` to terminal-specific code is `wrappers.rs`. Excluding it lets the rest of `color/` move cleanly. `TermColor` is the second consideration — it's defined in `color/mod.rs` but is conceptually a terminal-emission trait. It stays in biscuit-terminal alongside its impls.

## External Blast Radius

### Layout

50+ import sites across 11 areas: biscuit-terminal, biscuit-tui, biscuit-visualized, claudine, darkmatter, messenger, model-citizen, playa, sniff, schematic, unchained-ai. Every site imports types, not methods. **Zero behavioral changes required** if re-exports are in place.

### Color

21 import sites across 5 areas: claudine (cli + lib), darkmatter/cli, playa/cli, sniff/cli. Distribution:

| Symbol(s) | Sites |
|---|---|
| `Color` | most common |
| `Color, Tailwind` | 11 |
| `WebColor` | several |
| `BasicColor`, `RgbColor` | a few each |
| `TermColor` | 2 (the only sites that touch ANSI emission externally) |
| `Octet` | 1 |

The two `TermColor`-importing sites continue to work because `TermColor` and its impls stay in biscuit-terminal.

## Phasing

The work splits into four checkpoints, each independently verifiable (`cargo check`, `cargo test`, and a smoke test of `biscuit-terminal` examples). **Do not interleave** — each phase should land as a coherent commit/PR.

### Phase 0 — Preparation (no code moves yet)

1. **Lift the `TermColor` trait** out of `color/mod.rs` into its own module `biscuit-terminal/lib/src/utils/term_color.rs` (still in biscuit-terminal). This decouples the trait's location from the data types that will move.
    - `color/mod.rs` re-exports `TermColor` to preserve `use biscuit_terminal::utils::color::TermColor`.
    - All `impl TermColor for X` blocks stay in their current files for now.
2. **Spot-check `Tailwind::to_hdr_color`**: confirm the method doesn't call into any biscuit-terminal-only code (it should only use `HdrColor` constructors).
3. **Update `Cargo.toml`s**: add `renderable` to `biscuit-terminal/lib`'s deps. Confirm dependency direction (`biscuit-terminal` → `renderable`) is acyclic in `cargo metadata`.

**Exit criterion**: `just lint && just test` passes for biscuit-terminal and any caller; behavior unchanged.

### Phase 1 — Move Layout data

1. Create `renderable/src/layout.rs` (or a `layout/` submodule if it grows). Move:
    - `Alignment`, `Margin` (+ `add_chars`), `RowFill`, `MaxWidth`
    - `Layout` struct (fields), `Default`, `new`, `resolve_margin`, `available_width`
    - The portable doc-tests (those not exercising `apply_layout` / `apply_block_layout`)
2. Create `renderable/src/wrap_policy.rs` and move `WordWrap` there.
3. In `biscuit-terminal/lib/src/utils/layout.rs`:
    - Replace the moved items with `pub use renderable::layout::*;` (and `pub use renderable::wrap_policy::WordWrap;`).
    - Keep `apply_layout` / `apply_block_layout` as an **extension trait** `LayoutTerminalExt`:

      ```rust
      pub trait LayoutTerminalExt {
          fn apply_layout(&self, content: &str, terminal_width: u32) -> String;
          fn apply_block_layout(&self, content: &str, terminal_width: u32) -> String;
      }
      impl LayoutTerminalExt for Layout { /* moved bodies */ }
      ```

    - Re-export `LayoutTerminalExt` from `biscuit_terminal::prelude` so `layout.apply_layout(...)` keeps working with a `use biscuit_terminal::prelude::*;`.
    - Keep `RenderableWrapper` in place.
4. **Bridge the `page_bg_color: Option<Color>` field** for one phase by generic-parameterizing `Layout` over the color type: `pub struct Layout<C = ()> { …, page_bg_color: Option<C> }`. Inside biscuit-terminal alias `pub type Layout = renderable::layout::Layout<Color>;` so every caller's `Layout` type is byte-identical via the re-export.

   The generic is **transitional**. Phase 2 removes the type parameter and pins `page_bg_color: Option<renderable::color::Color>`.

**Exit criterion**: every `cargo check` across the workspace passes with **no changes outside `biscuit-terminal/lib` and `renderable`**. All biscuit-terminal tests pass; ANSI output snapshots for example components match prior bytes exactly.

### Phase 2 — Move Color data

1. In `renderable/src/color/` create the same file layout minus `wrappers.rs`:
    - `mod.rs` — exports, but `TermColor` is **not** present here
    - `octet.rs`, `basic.rs`, `rgb.rs`, `hdr.rs`, `web.rs`, `tailwind.rs`, `color_enum.rs`
2. Move the data items per the inventory. **Strip the `impl TermColor for X` blocks** from `basic.rs`, `rgb.rs`, `hdr.rs`, `web.rs` as you move them.
3. In `biscuit-terminal`, create `utils/color_terminal.rs` (or extend `utils/term_color.rs`) containing the four `impl TermColor for {BasicColor, RgbColor, HdrColor, WebColor}` blocks, importing the types from `renderable::color`.
4. In `biscuit-terminal/lib/src/utils/color/mod.rs`:
    - Delete the moved files (`basic.rs`, `rgb.rs`, …) from this directory.
    - Replace with re-exports:

      ```rust
      pub use renderable::color::{
          BasicColor, Color, FgBg, HdrColor, Octet, OctetError, RgbColor,
          Tailwind, WebColor, WEB_COLOR_LOOKUP, basic_color_to_rgb,
      };
      ```

    - Re-export `TermColor` from its biscuit-terminal location.
    - Keep `wrappers.rs` (depends on `RenderableWrapper` / `Terminal` and stays).
5. **Resolve the Phase 1 generic**: rewrite `renderable::layout::Layout` to use `Option<renderable::color::Color>` directly. Drop the type parameter. The `biscuit-terminal::utils::layout::Layout` re-export remains.

**Exit criterion**: full workspace `cargo check && cargo test`. Visual smoke test of one styled component (e.g. `BasicColor::Red.fg("hi")` and a `Stylesheet` round-trip) confirms ANSI output unchanged.

### Phase 3 — Cleanup and consolidation

1. Delete any transitional shim files in biscuit-terminal whose sole content is `pub use renderable::…` if module-path duplication has accumulated.
2. **Keep** `pub use renderable::layout::*` and `pub use renderable::color::*` in `biscuit_terminal::utils::{layout, color}` — these are the public-API stability guarantee for downstream crates. Do not delete them.
3. Add `renderable::prelude` exporting `Layout`, `Margin`, `Alignment`, `Color`, `RowFill`, `WordWrap`. Convenience only — not required for any caller.
4. Update `docs/dependencies.md` (workspace-level) and per-area docs (`darkmatter/docs/dependencies.md`, `claudine/docs/dependencies.md`, etc.) to reflect the new direct dependency on `renderable`.
5. Update `.claude/skills/biscuit-terminal/SKILL.md` and any biscuit-terminal docs that name `utils::layout::*` or `utils::color::*` as the canonical path. Mention `renderable::{layout, color}` as the source of truth; keep biscuit-terminal paths documented as compatibility re-exports.

**Exit criterion**: `cargo doc -p renderable -p biscuit-terminal --no-deps` produces no new warnings; intra-doc links resolve.

## API Surface in `renderable` after the move

```rust
// renderable/src/lib.rs
pub mod layout;
pub mod color;
pub mod stylesheet;       // already there from the Stylesheet move
pub mod wrap_policy;
pub mod prelude;

// renderable/src/layout.rs
pub use crate::wrap_policy::WordWrap;

pub struct Layout {
    pub left_margin: Margin,
    pub right_margin: Margin,
    pub top_margin: Margin,
    pub bottom_margin: Margin,
    pub alignment: Alignment,
    pub row_fill_strategy: RowFill,
    pub word_wrap: WordWrap,
    pub page_bg_color: Option<crate::color::Color>,
}
pub enum Alignment { Left, Center, Right }
pub enum Margin { None, Chars(u32), Percent(f32), Offset(Box<Margin>, u32) }
pub enum RowFill { Auto, Fill, Exact }
pub enum MaxWidth { None, Chars(u32), Percent(f32) }
impl Layout {
    pub fn new(...);
    pub fn resolve_margin(...);
    pub fn available_width(...);
}
impl Margin { pub fn add_chars(self, chars: u32) -> Margin }

// renderable/src/color/mod.rs
pub mod basic;
pub mod color_enum;
pub mod hdr;
pub mod octet;
pub mod rgb;
pub mod tailwind;
pub mod web;

pub use basic::{BasicColor, FgBg, basic_color_to_rgb};
pub use color_enum::Color;
pub use hdr::HdrColor;
pub use octet::{Octet, OctetError};
pub use rgb::RgbColor;
pub use tailwind::Tailwind;
pub use web::{WEB_COLOR_LOOKUP, WebColor};

// renderable/src/prelude.rs
pub use crate::layout::{Alignment, Layout, Margin, MaxWidth, RowFill, WordWrap};
pub use crate::color::{BasicColor, Color, HdrColor, RgbColor, Tailwind, WebColor};
```

## Re-export shims in `biscuit-terminal`

```rust
// biscuit-terminal/lib/src/utils/layout.rs (post-move)
pub use renderable::layout::{Alignment, Layout, Margin, MaxWidth, RowFill};
pub use renderable::wrap_policy::WordWrap;

pub trait LayoutTerminalExt {
    fn apply_layout(&self, content: &str, terminal_width: u32) -> String;
    fn apply_block_layout(&self, content: &str, terminal_width: u32) -> String;
}
impl LayoutTerminalExt for Layout { /* … */ }

pub trait RenderableWrapper { /* … unchanged … */ }
```

```rust
// biscuit-terminal/lib/src/utils/color/mod.rs (post-move)
pub use renderable::color::{
    BasicColor, Color, FgBg, HdrColor, Octet, OctetError, RgbColor,
    Tailwind, WebColor, WEB_COLOR_LOOKUP, basic_color_to_rgb,
};
pub use super::term_color::TermColor;     // local to biscuit-terminal
pub use super::color_terminal::*;          // local TermColor impls
pub use self::wrappers::{
    BasicColorWrapper, RgbColorWrapper, TailwindColorWrapper, WebColorWrapper,
};

pub mod wrappers;                          // stays — depends on Terminal
```

## Migration checklist for downstream callers

Ideal outcome: **no changes required**. Verification steps for each consuming area (claudine, darkmatter, playa, model-citizen, etc.):

1. `cargo check -p <area>` passes without any source edits.
2. `cargo test -p <area>` produces the same pass/fail set as before.
3. If a snapshot test asserts on ANSI bytes, confirm the byte sequences match — they must, since the `TermColor` impls did not change.
4. **Optional** (post-move, separately): area maintainers may update imports to prefer `renderable::{layout, color}::…` over `biscuit_terminal::utils::{layout, color}::…` for new code. Existing imports do not need updating.

## Verification

For each phase:

```bash
just lint
just test          # workspace-wide
just doctest       # all crates with doc-tests
cargo build -p biscuit-terminal --example <one canonical example>
diff <(./target/debug/examples/<example>) <golden snapshot>
```

Specific cross-phase sanity tests:

- A `Stylesheet` with `CssColor::Rgb(51, 102, 153)` round-trips through `to_css` unchanged.
- A `Layout { page_bg_color: Some(Color::Tailwind(Tailwind::Blue500)), ..Default::default() }` produces the same `apply_block_layout` output before and after.
- `BasicColor::Red.fg("hi")` produces identical bytes before and after Phase 2.

## Risk Register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Hidden cyclic dep through `serde` derive expansion or doc-test paths | Low | Phase 0 `cargo metadata` check; doc-tests use `ignore` until paths settle |
| `wrappers.rs` accidentally pulled into renderable | Low | Excluded by inventory; CI grep guard: `! grep -r 'use crate::terminal' renderable/src` |
| Generic `Layout<C>` leaks into a caller signature in Phase 1 | Medium | Re-export uses default type param; if a caller imports `Layout` directly from `renderable` during Phase 1 they'd see the generic — discouraged until Phase 2 finishes. Hold all `renderable::layout` imports until Phase 2. |
| ANSI byte drift due to inadvertent `Display` / `fmt` change | Low | Snapshot tests of representative styled output before/after each phase |
| Tailwind / Web color tables silently re-indexed | Low | Move files as-is; do not reformat enum variant order |
| Stylesheet spec hasn't landed yet | High if mis-sequenced | Hard prereq; gate this work on Stylesheet PR merging |
| `TermColor` becomes a trait orphan (impls in one crate, trait in another, callers in a third) | Medium | Phase 0 keeps trait + impls colocated in biscuit-terminal. Verify with: every `TermColor` impl lives in same crate as the trait — never in `renderable`. |

## Open Questions

1. **Should `Octet` / `OctetError` move?** Inventory says yes. Confirm no external caller treats `OctetError` as a biscuit-terminal-rooted error in `Box<dyn Error>` chains where the source crate matters.
2. **Re-export shim policy**: do we keep `pub use renderable::…` in `biscuit_terminal::utils::{layout, color}` indefinitely, or mark them `#[deprecated]` after one minor version? Recommendation: keep indefinitely — they cost nothing and preserve a stable surface.
3. **Tailwind `to_hdr_color`**: lives in `tailwind.rs` and produces `HdrColor`. If Tailwind is ever extracted into its own crate (out of scope here), the dependency direction would need revisiting. Not a blocker.
4. **`prelude` semantics**: should `renderable::prelude` shadow `biscuit_terminal::prelude` for layout/color items? Probably yes (cleaner imports for new code), but verify no symbol collisions.
5. **Backwards compat for `Color: Serialize / Deserialize`**: `Color` derives `Serialize, Deserialize`. The discriminant names must not change during the move — serialized payloads (e.g. in claudine config files) must continue to deserialize. Confirm enum variant names stay byte-identical.

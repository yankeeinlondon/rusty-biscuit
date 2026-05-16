# Project 3: Using the new API Surface — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the three stylesheet types per Scheme A, delete the deprecated `BrowserRenderable` methods, and migrate every caller and existing implementor onto the new `render_html_fragment` / `render_html_page` API surface.

**Architecture:** `renderable` owns the cross-target rendering types. Three stylesheet concepts are renamed to CSS-accurate names: the declaration block becomes `CssStyle`, a `(selector, block)` pair becomes the new `CssRule` struct, and the rule collection becomes `Stylesheet` (previously `HtmlStyleSheet`). Components implement `render_html_fragment` (returning `BrowserFragment<Ready>`) instead of the deprecated string-producing methods; `DarkmatterPage` stops being a `BrowserRenderable` and becomes a plain `HtmlPage` builder.

**Tech Stack:** Rust, Cargo workspace (rusty-biscuit monorepo)

**Prerequisites:** Project 1 and Project 2 complete.

After Project 1, the `BrowserRenderable` trait, `BrowserFragment`, `ComposableNode`, `HtmlStyleSheet`, `ComponentStylesheet`, `PageOptions`, `HtmlPage`, and the declaration-block `Stylesheet` struct all live in the `renderable` crate. After Project 2, `BrowserRenderable` carries four methods (`render_to_browser`, `render_to_browser_with_inline_variables`, `render_html_fragment`, `render_html_page`), `ComposableNode` has a `RawHtml(String)` variant, and `BrowserFragment` exposes a `define_as_raw_html` builder. This plan assumes that target state. If a referenced symbol is absent, Project 1 or Project 2 is incomplete — stop and resolve that first.

> **Build discipline:** NEVER run `cargo build` at the repo root. Always scope with `-p <pkg>`. Test runner is `cargo nextest run -p <pkg>` (fallback `cargo test -p <pkg>`). Crate package names: `renderable`, `biscuit-terminal` (lib crate name `biscuit_terminal`), `darkmatter`.

---

## Phase 1: Stylesheet Naming Housekeeping (Scheme A)

Scheme A renames three concepts. Do them in dependency order so the compiler guides each step:

1. Introduce `CssRule` (new struct wrapping `(selector, CssStyle)`).
2. Rename declaration block `Stylesheet` → `CssStyle`.
3. Rename collection `HtmlStyleSheet` → `Stylesheet`.

`ClassDefinition` and the `LinkRel::Stylesheet` / `RelAttribute` enum variant are **left alone** — the latter is a CSS `rel` keyword, not a type.

### Task 1.1: Rename the declaration-block struct `Stylesheet` → `CssStyle`

The declaration-block struct (a list of `property: value` pairs, also used for inline `style=""`) was moved into `renderable` by Project 1. Locate its module first.

**Files:**
- Modify: the `renderable` source file defining `pub struct Stylesheet` (the declaration-block struct moved from `darkmatter/lib/src/render/stylesheet.rs` by Project 1 — find via the grep step below)
- Modify: `renderable/src/html/tag/mod.rs` (uses `Style(Stylesheet)` at the `HtmlAttribute` enum)
- Modify: `renderable/src/browser/mod.rs` (the `HtmlStyleSheet` tuple holds `Stylesheet`)
- Modify: any other `renderable` file importing the declaration-block `Stylesheet`

Steps:
- [ ] Run `grep -rn "Stylesheet" renderable/src --include="*.rs"` and `grep -rln "pub struct Stylesheet" renderable/src --include="*.rs"`. Identify the file defining the **declaration-block** `pub struct Stylesheet` (its doc comment describes "a list of `property: value` pairs", `to_css`, `to_terminal_string`). Record that path; call it `<css-style-file>`.
- [ ] In `<css-style-file>`, rename `pub struct Stylesheet` to `pub struct CssStyle`. Rename every `impl Stylesheet`, `impl ... for Stylesheet`, `Stylesheet::new`, `Stylesheet {` literal, and `-> Stylesheet` / `: Stylesheet` annotation **within that file** to `CssStyle`. Update the module-level and item-level doc comments that say "A `Stylesheet`" to "A `CssStyle`". Do NOT touch `StylesheetError` — leave that type name as-is (it is the declaration-block parse error and Scheme A does not rename it).
- [ ] In `renderable/src/html/tag/mod.rs`: change the enum variant `Style(Stylesheet)` to `Style(CssStyle)`. Update its doc line `/// `style` — inline CSS declarations.` if it names the type. Add/adjust the import: the `use` block must bring `CssStyle` into scope from `<css-style-file>`'s module path (replace any `Stylesheet` in the existing `use` path).
- [ ] In `renderable/src/browser/mod.rs`: in `pub struct HtmlStyleSheet(Vec<(String, Stylesheet)>)` change `Stylesheet` to `CssStyle`. In `HtmlStyleSheet::push` change the parameter `sheet: Stylesheet` to `sheet: CssStyle`. In `HtmlStyleSheet::entries` change the return type `&[(String, Stylesheet)]` to `&[(String, CssStyle)]`. Update the doc comment line `Each entry is a `(selector, Stylesheet)` pair.` to `Each entry is a `(selector, CssStyle)` pair.` and `A `CssStyle` declaration block` references in other doc comments accordingly. Add a `use` for `CssStyle` if `browser/mod.rs` does not already import the declaration-block type.
- [ ] Run `grep -rn "\bStylesheet\b" renderable/src --include="*.rs"`. Every remaining hit must now be either `HtmlStyleSheet`, `ComponentStylesheet`, `StylesheetError`, or `LinkRel::Stylesheet` / `RelAttribute::Stylesheet`. If a bare declaration-block `Stylesheet` remains, rename it to `CssStyle`.
- [ ] Run `cargo build -p renderable` (expected: compiles cleanly, no errors).
- [ ] Run `cargo nextest run -p renderable` (expected: all tests pass; fallback `cargo test -p renderable`).
- [ ] Commit: `git add -A && git commit -m "refactor(renderable): rename declaration-block Stylesheet to CssStyle"`

### Task 1.2: Introduce the `CssRule` struct

A `CssRule` is one `selector { block }` rule — the `(String, CssStyle)` pair given a name. The collection currently stores raw tuples; `CssRule` replaces those tuples.

**Files:**
- Modify: `renderable/src/browser/mod.rs` (define `CssRule`, rework `HtmlStyleSheet` to hold `Vec<CssRule>`)
- Test: `renderable/src/browser/mod.rs` (`#[cfg(test)]` module — add if absent)

Steps:
- [ ] In `renderable/src/browser/mod.rs`, immediately above `pub struct HtmlStyleSheet`, add the `CssRule` definition:
  ```rust
  /// A single CSS rule: a `selector { block }` pairing.
  ///
  /// The selector is stored verbatim as a string (the rendering layer emits
  /// it unchanged); the block is a [`CssStyle`] declaration list.
  #[derive(Debug, Clone)]
  pub struct CssRule {
      /// The CSS selector, emitted verbatim (e.g. `.simple-table .col-string`).
      pub selector: String,
      /// The declaration block applied to `selector`.
      pub block: CssStyle,
  }

  impl CssRule {
      /// Construct a rule pairing `selector` with `block`.
      pub fn new(selector: impl Into<String>, block: CssStyle) -> CssRule {
          CssRule {
              selector: selector.into(),
              block,
          }
      }
  }
  ```
  (If `CssStyle` does not derive `Debug` and `Clone`, drop the `#[derive(Debug, Clone)]` line on `CssRule` to match — verify by checking `<css-style-file>` from Task 1.1.)
- [ ] Change the field of `pub struct HtmlStyleSheet` from `Vec<(String, CssStyle)>` to `Vec<CssRule>`.
- [ ] Update `HtmlStyleSheet::new` body `HtmlStyleSheet(Vec::new())` — unchanged, still valid.
- [ ] Update `HtmlStyleSheet::push`: keep the signature `pub fn push(&mut self, selector: impl Into<String>, sheet: CssStyle) -> &mut Self` but change the body to `self.0.push(CssRule::new(selector, sheet)); self`.
- [ ] Update `HtmlStyleSheet::entries`: change return type from `&[(String, CssStyle)]` to `&[CssRule]`; body `&self.0` is unchanged.
- [ ] Update the `HtmlStyleSheet` doc comment: change `Each entry is a `(selector, CssStyle)` pair.` to `Each entry is a [`CssRule`] (a `(selector, CssStyle)` pairing).`
- [ ] Run `grep -rn "\.entries()" renderable/src --include="*.rs"`. For every call site that destructures the old tuple (e.g. `for (selector, block) in sheet.entries()`), update it to `for rule in sheet.entries()` and use `rule.selector` / `rule.block`.
- [ ] In `renderable/src/browser/mod.rs`, ensure a `#[cfg(test)] mod tests` block exists; add this test:
  ```rust
  #[cfg(test)]
  mod css_rule_tests {
      use super::*;

      #[test]
      fn push_stores_a_css_rule_with_selector() {
          let mut sheet = HtmlStyleSheet::new();
          sheet.push(".wrapper", CssStyle::new());
          let entries = sheet.entries();
          assert_eq!(entries.len(), 1);
          assert_eq!(entries[0].selector, ".wrapper");
      }
  }
  ```
  (If `CssStyle::new()` is not the constructor, use the actual constructor confirmed in Task 1.1's `<css-style-file>`.)
- [ ] Run `cargo nextest run -p renderable css_rule_tests` (expected: `push_stores_a_css_rule_with_selector` PASSES). If `cargo nextest` is unavailable, run `cargo test -p renderable css_rule_tests`.
- [ ] Run `cargo build -p renderable` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p renderable` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "refactor(renderable): introduce CssRule struct for selector-block pairs"`

### Task 1.3: Rename the collection struct `HtmlStyleSheet` → `Stylesheet`

The collection of `CssRule`s is what a stylesheet actually is, so it claims the `Stylesheet` name.

**Files:**
- Modify: `renderable/src/browser/mod.rs` (definition of `HtmlStyleSheet`, `ComponentStylesheet`, `PageOptions`)
- Modify: `renderable/src/html/mod.rs` (`HtmlPage` struct + methods import and use `HtmlStyleSheet`)
- Modify: any other `renderable` file importing `HtmlStyleSheet`

Steps:
- [ ] Run `grep -rn "HtmlStyleSheet" renderable/src --include="*.rs"` and record every file. Expected hits: `renderable/src/browser/mod.rs` and `renderable/src/html/mod.rs` (verify; act on whatever the grep reports).
- [ ] In `renderable/src/browser/mod.rs`: rename `pub struct HtmlStyleSheet` to `pub struct Stylesheet`. Rename every `impl HtmlStyleSheet`, `HtmlStyleSheet::new`, `HtmlStyleSheet(` tuple constructor, and `-> HtmlStyleSheet` / `: HtmlStyleSheet` annotation to `Stylesheet`. In `ComponentStylesheet`, the field `style: HtmlStyleSheet` becomes `style: Stylesheet`, and `ComponentStylesheet::new`'s body `style: HtmlStyleSheet::new()` becomes `style: Stylesheet::new()`. In `ComponentStylesheet::as_stylesheet`, the return type `-> HtmlStyleSheet` becomes `-> Stylesheet`. In `PageOptions`, the field `stylesheet: Option<HtmlStyleSheet>` becomes `stylesheet: Option<Stylesheet>`. Update doc comments referencing `HtmlStyleSheet` (e.g. `as_stylesheet` doc, `ComponentStylesheet` doc) to say `Stylesheet`.
- [ ] In `renderable/src/html/mod.rs`: update the `use crate::{ browser::{BrowserFragment, CodeFeature, HtmlStyleSheet}, ... }` import to `HtmlStyleSheet` → `Stylesheet`. In `pub struct HtmlPage`, the field `stylesheet: HtmlStyleSheet` becomes `stylesheet: Stylesheet`. In `impl Default for HtmlPage`, `stylesheet: HtmlStyleSheet::new()` becomes `stylesheet: Stylesheet::new()`. In `HtmlPage::new`, the parameter `style: Option<HtmlStyleSheet>` becomes `style: Option<Stylesheet>` and the body `style.unwrap_or(HtmlStyleSheet::new())` becomes `style.unwrap_or(Stylesheet::new())`.
- [ ] Run `grep -rn "HtmlStyleSheet" renderable/src --include="*.rs"` (expected: zero hits).
- [ ] Run `grep -rn "\bStylesheet\b" renderable/src --include="*.rs"`. Confirm every hit is now the collection type (`Stylesheet`), `ComponentStylesheet`, or `StylesheetError` — no stray declaration-block `Stylesheet` (those are `CssStyle` after Task 1.1) and no `HtmlStyleSheet`.
- [ ] Run `cargo build -p renderable` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p renderable` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "refactor(renderable): rename HtmlStyleSheet collection to Stylesheet"`

### Task 1.4: Re-export the renamed types and propagate to downstream crates

`biscuit-terminal` and `darkmatter` consume these types. After the renames, fix their imports.

**Files:**
- Modify: `renderable/src/lib.rs` and/or `renderable/src/browser/mod.rs` (re-exports, if any are public-facing)
- Modify: every `biscuit-terminal` / `darkmatter` file importing `Stylesheet`, `HtmlStyleSheet`, or the declaration-block type

Steps:
- [ ] Run `grep -rn "HtmlStyleSheet\|renderable::.*Stylesheet\|use renderable" biscuit-terminal darkmatter --include="*.rs"`. Record every file importing a renamed type from `renderable`.
- [ ] For each such file: replace `HtmlStyleSheet` with `Stylesheet` in `use` paths and type annotations; replace any import of the declaration-block `Stylesheet` with `CssStyle`. A file that imports both the collection and the declaration block must list both `CssStyle` and `Stylesheet` in its `use` group.
- [ ] If `renderable/src/lib.rs` or `renderable/src/browser/mod.rs` re-exports any of these types with `pub use`, update the re-export names to `CssStyle`, `CssRule`, `Stylesheet`.
- [ ] Run `cargo build -p biscuit-terminal` (expected: compiles, or fails only with caller-migration errors handled in Phase 2 — at this point the stylesheet renames themselves must produce no `unresolved import` errors).
- [ ] Run `cargo build -p darkmatter` (expected: same — no `unresolved import` errors for stylesheet types).
- [ ] Run `cargo nextest run -p renderable` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "refactor: propagate stylesheet renames to biscuit-terminal and darkmatter"`

---

## Phase 2: Migrate the Three String-Producing Implementors

`HorizontalRule`, `GraphExpression`, and `YamlBlock` each currently implement `render_to_browser` (and `HorizontalRule` also `render_to_browser_with_inline_variables`). Per `decisions.md` item 12B, each gains a one-line `render_html_fragment` that wraps the existing string output in a `RawHtml` node. Project 2 added the `define_as_raw_html` builder and the `ComposableNode::RawHtml` variant; this phase uses them.

> The `render_to_browser` bodies are NOT deleted here — they are still needed as the source string for `define_as_raw_html`, and Phase 3 deletes the trait methods. Phase 2 only **adds** `render_html_fragment` (and `render_html_page`).

### Task 2.1: Migrate `HorizontalRule`

**Files:**
- Modify: `biscuit-terminal/lib/src/components/horizontal_rule/browser.rs`
- Test: `biscuit-terminal/lib/src/components/horizontal_rule/browser.rs` (`#[cfg(test)]` module)

Steps:
- [ ] Open `biscuit-terminal/lib/src/components/horizontal_rule/browser.rs`. Confirm the `impl BrowserRenderable for HorizontalRule` block currently defines `render_to_browser`, `render_to_browser_with_inline_variables`, and `as_any`.
- [ ] Confirm the `use` line imports `BrowserRenderable` from `renderable` (Project 1 moved the trait into `renderable/src/browser/renderable.rs`, re-exported at `renderable::browser::BrowserRenderable`). If it still says `use crate::components::renderable::BrowserRenderable;`, that means Project 1 is incomplete — stop and resolve. Otherwise the import should be `use renderable::browser::BrowserRenderable;` (verify the exact path against `renderable/src/browser/mod.rs`).
- [ ] Add the necessary imports for the fragment API. At the top of `browser.rs`, ensure these are in scope (add a `use` line if missing): `use renderable::browser::fragment::{BrowserFragment, Ready};` and `use renderable::html::HtmlPage;` and `use renderable::browser::PageOptions;`. Verify each path against the actual module tree in `renderable/src` before writing.
- [ ] Inside `impl BrowserRenderable for HorizontalRule`, add `render_html_fragment` immediately after `render_to_browser_with_inline_variables`:
  ```rust
  /// Promote the rule to a [`BrowserFragment`].
  ///
  /// The SVG is emitted as a [`ComposableNode::RawHtml`] island — it is a
  /// prebuilt string and the renderer must not escape it. The `var(--hr-*)`
  /// custom-property placeholders stay literal in the SVG; the page declares
  /// the variables.
  fn render_html_fragment(&self) -> BrowserFragment<Ready> {
      BrowserFragment::new()
          .define_as_raw_html(self.render_to_browser())
          .finalize()
  }
  ```
- [ ] Immediately after `render_html_fragment`, add `render_html_page`:
  ```rust
  /// Promote this single rule to a standalone [`HtmlPage`].
  fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
      let mut html_page = HtmlPage::from(self.render_html_fragment());
      if let Some(options) = page {
          html_page.apply_page_options(options);
      }
      html_page
  }
  ```
  (If Project 2 named the options-application method differently than `apply_page_options`, use the actual name — confirm by grepping `renderable/src/html/mod.rs` for `PageOptions`.)
- [ ] In the `#[cfg(test)]` module of `browser.rs` (add the module if absent), add a failing-first test:
  ```rust
  #[test]
  fn render_html_fragment_wraps_svg_as_raw_html() {
      use renderable::browser::fragment::ComposableNode;
      let rule = HorizontalRule::new();
      let fragment = rule.render_html_fragment();
      let html = fragment.render();
      assert!(html.contains("<svg"), "fragment must contain the SVG markup");
  }
  ```
  (If `BrowserFragment<Ready>::render` is not yet implemented — Project 1/2 left it `todo!()` — instead assert on the raw-html string directly by checking `rule.render_to_browser().contains("<svg")` plus that `render_html_fragment` does not panic. Prefer the `render()` assertion if `render()` is implemented; verify by reading `renderable/src/browser/fragment.rs`.)
- [ ] Run `cargo nextest run -p biscuit-terminal render_html_fragment_wraps_svg_as_raw_html` (expected before adding the method: FAIL with a missing-method error; after adding: PASS). Fallback: `cargo test -p biscuit-terminal render_html_fragment_wraps_svg_as_raw_html`.
- [ ] Run `cargo build -p biscuit-terminal` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p biscuit-terminal` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "feat(biscuit-terminal): migrate HorizontalRule to render_html_fragment"`

### Task 2.2: Migrate `GraphExpression`

**Files:**
- Modify: `biscuit-terminal/lib/src/components/graph_expression.rs`
- Test: `biscuit-terminal/lib/src/components/graph_expression.rs` (`#[cfg(test)]` module)

Steps:
- [ ] Open `biscuit-terminal/lib/src/components/graph_expression.rs`. Confirm `impl BrowserRenderable for GraphExpression` defines `render_to_browser` and `as_any`.
- [ ] At the top of the file, ensure `BrowserFragment`, `Ready`, `HtmlPage`, and `PageOptions` are imported from `renderable` (add `use` lines if missing — use the same paths confirmed in Task 2.1).
- [ ] Inside `impl BrowserRenderable for GraphExpression`, add `render_html_fragment` immediately after `render_to_browser`:
  ```rust
  /// Promote the graph to a [`BrowserFragment`].
  ///
  /// The SVG is emitted as a [`ComposableNode::RawHtml`] island — a prebuilt
  /// string that the renderer must not escape. The graph carries no scripts
  /// and no external dependencies.
  fn render_html_fragment(&self) -> BrowserFragment<Ready> {
      BrowserFragment::new()
          .define_as_raw_html(self.render_to_browser())
          .finalize()
  }
  ```
- [ ] Immediately after, add `render_html_page`:
  ```rust
  /// Promote this single graph to a standalone [`HtmlPage`].
  fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
      let mut html_page = HtmlPage::from(self.render_html_fragment());
      if let Some(options) = page {
          html_page.apply_page_options(options);
      }
      html_page
  }
  ```
- [ ] In the `#[cfg(test)]` module, add:
  ```rust
  #[test]
  fn render_html_fragment_wraps_graph_output() {
      let graph = GraphExpression::try_from("digraph { a -> b }").unwrap();
      let fragment = graph.render_html_fragment();
      let html = fragment.render();
      assert!(!html.is_empty(), "fragment render must produce output");
  }
  ```
  (Use whatever constructor the existing tests in this file use to build a `GraphExpression` — confirm by reading the file's test module. If `render()` is `todo!()`, fall back to asserting `graph.render_html_fragment()` does not panic and `graph.render_to_browser()` is non-empty, as in Task 2.1.)
- [ ] Run `cargo nextest run -p biscuit-terminal render_html_fragment_wraps_graph_output` (expected before adding the method: FAIL; after: PASS).
- [ ] Run `cargo build -p biscuit-terminal` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p biscuit-terminal` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "feat(biscuit-terminal): migrate GraphExpression to render_html_fragment"`

### Task 2.3: Migrate `YamlBlock`

**Files:**
- Modify: `darkmatter/lib/src/markdown/yaml_block.rs`
- Test: `darkmatter/lib/src/markdown/yaml_block.rs` (`#[cfg(test)]` module)

Steps:
- [ ] Open `darkmatter/lib/src/markdown/yaml_block.rs`. Confirm `impl BrowserRenderable for YamlBlock` defines `render_to_browser` and `as_any`.
- [ ] Confirm the import line. It currently reads `use biscuit_terminal::components::renderable::{BrowserRenderable, Renderable};`. After Project 1, `BrowserRenderable` lives in `renderable`; the import must split: `Renderable` (now `TerminalRenderable` per Project 1) stays sourced from wherever Project 1 placed it, and `BrowserRenderable` is imported from `renderable`. If Project 1 left this import pointing at `biscuit_terminal`, Project 1 is incomplete — stop and resolve.
- [ ] Add imports for `BrowserFragment`, `Ready`, `HtmlPage`, `PageOptions` from `renderable` (same paths as Task 2.1; add `use` lines if missing).
- [ ] Inside `impl BrowserRenderable for YamlBlock`, add `render_html_fragment` immediately after `render_to_browser`:
  ```rust
  /// Promote the YAML block to a [`BrowserFragment`].
  ///
  /// The highlighted code block is emitted as a [`ComposableNode::RawHtml`]
  /// island — a prebuilt string the renderer must not escape. A dedicated
  /// `CssStyle` may be added later if syntax highlighting moves into the
  /// fragment model.
  fn render_html_fragment(&self) -> BrowserFragment<Ready> {
      BrowserFragment::new()
          .define_as_raw_html(self.render_to_browser())
          .finalize()
  }
  ```
- [ ] Immediately after, add `render_html_page`:
  ```rust
  /// Promote this single YAML block to a standalone [`HtmlPage`].
  fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
      let mut html_page = HtmlPage::from(self.render_html_fragment());
      if let Some(options) = page {
          html_page.apply_page_options(options);
      }
      html_page
  }
  ```
- [ ] In the `#[cfg(test)]` module, add:
  ```rust
  #[test]
  fn render_html_fragment_wraps_yaml_output() {
      let block = YamlBlock::new("key: value").unwrap();
      let fragment = block.render_html_fragment();
      let html = fragment.render();
      assert!(html.contains("value"), "fragment must contain the YAML body");
  }
  ```
  (If `render()` is `todo!()`, fall back to asserting `block.render_html_fragment()` does not panic and `block.render_to_browser().contains("value")`.)
- [ ] Run `cargo nextest run -p darkmatter render_html_fragment_wraps_yaml_output` (expected before adding the method: FAIL; after: PASS).
- [ ] Run `cargo build -p darkmatter` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p darkmatter` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "feat(darkmatter): migrate YamlBlock to render_html_fragment"`

---

## Phase 3: Drop `DarkmatterPage` from `BrowserRenderable`

Per `decisions.md` item 12A, `DarkmatterPage` is a multi-fragment page assembler, not a single-fragment component. It stops implementing `BrowserRenderable` and becomes a plain `HtmlPage` builder. Its own inherent `render_to_browser(&self, md: &Markdown)` method (the one taking a `Markdown` argument, at `darkmatter/lib/src/layout/page.rs:475`) is **kept** — only the trait `impl` is removed.

### Task 3.1: Remove the `BrowserRenderable` impl from `DarkmatterPage`

**Files:**
- Modify: `darkmatter/lib/src/layout/page.rs`

Steps:
- [ ] Open `darkmatter/lib/src/layout/page.rs`. Locate `impl BrowserRenderable for DarkmatterPage` (around line 728). It defines a zero-argument `render_to_browser(&self) -> String` and `as_any`.
- [ ] Delete the entire `impl BrowserRenderable for DarkmatterPage { ... }` block. Do **not** delete the inherent `impl DarkmatterPage` method `render_to_browser(&self, md: &Markdown) -> Result<String, PageRenderError>` near line 475 — that is a different method (different signature, takes `&Markdown`) and remains the public API.
- [ ] In the file's `use` imports, `BrowserRenderable` is now unused. Remove `BrowserRenderable` from the `use biscuit_terminal::components::renderable::{BrowserRenderable, Renderable};` line (or from wherever Project 1 re-pointed it), keeping `Renderable` / `TerminalRenderable`.
- [ ] Run `grep -rn "BrowserRenderable" darkmatter/lib/src/layout/page.rs` (expected: zero hits).
- [ ] Run `cargo build -p darkmatter` (expected: compiles, OR fails only at the call sites covered by Task 3.2 — if it fails elsewhere, investigate before continuing).
- [ ] Commit (after Task 3.2 if build still fails — see below).

### Task 3.2: Fix callers that relied on `DarkmatterPage: BrowserRenderable`

**Files:**
- Modify: any `darkmatter` file invoking `BrowserRenderable` methods on a `DarkmatterPage`, or treating it as `dyn BrowserRenderable`

Steps:
- [ ] Run `grep -rn "DarkmatterPage" darkmatter --include="*.rs" | grep -i "browser\|dyn BrowserRenderable\|render_to_browser\|render_html"`. Inspect each hit.
- [ ] For any call site that called the zero-argument trait `render_to_browser()` on a `DarkmatterPage`, change it to call the inherent `render_to_browser(&markdown)` (passing the `Markdown` the page already holds — the inherent method returns `Result<String, PageRenderError>`, so handle the `Result`). If the call site genuinely needed a `dyn BrowserRenderable`, it must instead obtain the `HtmlPage` via the assembler path; route it through the inherent method.
- [ ] Run `grep -rn "DarkmatterPage" darkmatter/lib/tests darkmatter/cli --include="*.rs" 2>/dev/null | grep -i browser`. Update any integration test or CLI code that downcast a `DarkmatterPage` through `BrowserRenderable` or called its trait method.
- [ ] Run `cargo build -p darkmatter` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p darkmatter` (expected: all tests pass; fallback `cargo test -p darkmatter`).
- [ ] Commit: `git add -A && git commit -m "refactor(darkmatter): drop BrowserRenderable impl from DarkmatterPage"`

---

## Phase 4: Delete the Deprecated Trait Methods

With every implementor migrated, remove `render_to_browser` and `render_to_browser_with_inline_variables` from the `BrowserRenderable` trait and from all `impl` blocks.

### Task 4.1: Remove the deprecated methods from the trait definition

**Files:**
- Modify: `renderable/src/browser/renderable.rs` (the `BrowserRenderable` trait definition)

Steps:
- [ ] Open `renderable/src/browser/renderable.rs`. Locate `pub trait BrowserRenderable`. After Project 2 it carries four methods.
- [ ] Delete the method declarations `fn render_to_browser(&self) -> String;` and `fn render_to_browser_with_inline_variables(&self, variables: &HashMap<String, String>) -> String;` (including any default-body implementations and doc comments). Keep `render_html_fragment` and `render_html_page`. Keep `as_any` if the trait declares it.
- [ ] If the trait file's only use of `std::collections::HashMap` was the deleted method signature, remove the now-unused `use std::collections::HashMap;` import.
- [ ] Confirm the final trait reads exactly (allowing for `as_any`):
  ```rust
  pub trait BrowserRenderable: std::fmt::Debug + Any {
      fn render_html_fragment(&self) -> BrowserFragment<Ready>;
      fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage;
  }
  ```
  (If the trait already had `as_any`, preserve it. If `decisions.md` item 2's signature shows no `as_any`, match `decisions.md` — it is authoritative.)
- [ ] The trait file references `BrowserFragment`, `Ready`, `PageOptions`, and `HtmlPage`. Confirm the existing `use` block at the top of `renderable/src/browser/renderable.rs` still imports those (Project 2 added them); the deletion of the two string methods does not change those imports.
- [ ] Run `cargo build -p renderable` (expected: compiles cleanly — no impl in `renderable` itself relies on the deleted methods).
- [ ] Commit (after Task 4.2 if downstream build fails).

### Task 4.2: Remove the deprecated method bodies from the three implementors

The three components keep their string-producing logic, but it is no longer a *trait* method. Per `decisions.md` item 2B, no per-component variable hook survives, so `render_to_browser_with_inline_variables` is deleted outright. The `render_to_browser` body that `render_html_fragment` depends on is converted from a trait method into a **private inherent method** so `render_html_fragment` still has its source string.

**Files:**
- Modify: `biscuit-terminal/lib/src/components/horizontal_rule/browser.rs`
- Modify: `biscuit-terminal/lib/src/components/graph_expression.rs`
- Modify: `darkmatter/lib/src/markdown/yaml_block.rs`

Steps:
- [ ] In `biscuit-terminal/lib/src/components/horizontal_rule/browser.rs`: delete `render_to_browser_with_inline_variables` entirely (body and doc comment). Move `render_to_browser` **out** of the `impl BrowserRenderable for HorizontalRule` block into a separate `impl HorizontalRule` block, renaming it to a private inherent method `fn render_browser_svg(&self) -> String` (keep the body verbatim). Update `render_html_fragment` to call `self.render_browser_svg()` instead of `self.render_to_browser()`. The `impl BrowserRenderable for HorizontalRule` block now contains only `render_html_fragment`, `render_html_page`, and `as_any` (if present).
- [ ] In `biscuit-terminal/lib/src/components/graph_expression.rs`: move `render_to_browser` out of `impl BrowserRenderable for GraphExpression` into an `impl GraphExpression` block, renaming it to `fn render_browser_svg(&self) -> String` (body verbatim, including the `browser_fallback` calls). Update `render_html_fragment` to call `self.render_browser_svg()`. The trait `impl` block keeps only `render_html_fragment`, `render_html_page`, and `as_any` (if present).
- [ ] In `darkmatter/lib/src/markdown/yaml_block.rs`: move `render_to_browser` out of `impl BrowserRenderable for YamlBlock` into an `impl YamlBlock` block, renaming it to `fn render_browser_html(&self) -> String` (body verbatim). Update `render_html_fragment` to call `self.render_browser_html()`. The trait `impl` block keeps only `render_html_fragment`, `render_html_page`, and `as_any` (if present).
- [ ] Run `grep -rn "render_to_browser\b" biscuit-terminal/lib/src darkmatter/lib/src --include="*.rs"`. The only legitimate remaining hit is the inherent `DarkmatterPage::render_to_browser(&self, md: &Markdown)` in `darkmatter/lib/src/layout/page.rs`. Any other hit is a missed reference — fix it (test code calling the old method must call the new inherent method or `render_html_fragment`).
- [ ] Run `grep -rn "render_to_browser_with_inline_variables" biscuit-terminal darkmatter renderable --include="*.rs"` (expected: zero hits).
- [ ] Run `cargo build -p renderable -p biscuit-terminal -p darkmatter` (expected: all three compile cleanly).
- [ ] Run `cargo nextest run -p renderable -p biscuit-terminal -p darkmatter` (expected: all tests pass; fallback `cargo test` per crate).
- [ ] Commit: `git add -A && git commit -m "refactor: remove deprecated render_to_browser methods from BrowserRenderable"`

### Task 4.3: Update existing tests that exercised the deprecated methods

**Files:**
- Modify: `biscuit-terminal/lib/src/components/graph_expression.rs` (tests at ~L547, ~L566 call `render_to_browser`)
- Modify: any other test invoking `render_to_browser` / `render_to_browser_with_inline_variables`

Steps:
- [ ] Run `grep -rn "render_to_browser\|render_to_browser_with_inline_variables" biscuit-terminal/lib/tests biscuit-terminal/lib/src darkmatter/lib/tests darkmatter/lib/src --include="*.rs"` and inspect every hit inside a `#[cfg(test)]` block or `tests/` file.
- [ ] For each test that called the trait `render_to_browser()`, replace it with `render_html_fragment()` and assert on the rendered fragment (`.render()` if implemented, otherwise on the private inherent `render_browser_svg()` / `render_browser_html()` — those are crate-private, so in-module tests can call them, but `tests/` integration tests cannot and must use `render_html_fragment().render()`).
- [ ] For any test of `render_to_browser_with_inline_variables` (the variable-substitution behavior), delete it — `decisions.md` item 2B removed that mechanism; components now emit `var(--foo)` literally and the page declares variables. If a replacement assertion is valuable, add a test confirming `render_html_fragment().render()` contains the literal `var(--hr-` token.
- [ ] Run `cargo nextest run -p biscuit-terminal -p darkmatter` (expected: all tests pass).
- [ ] Run `cargo build -p biscuit-terminal -p darkmatter` (expected: compiles cleanly).
- [ ] Commit: `git add -A && git commit -m "test: update browser-rendering tests for the new fragment API"`

---

## Phase 5: biscuit-terminal HTML-Page Example (TDD)

`spec.md` Project 3 requires `biscuit-terminal` to compile a representative example that uses components to build an HTML page. Build it test-first.

### Task 5.1: Write the failing example-driving test

**Files:**
- Create: `biscuit-terminal/lib/tests/html_page_example.rs`

Steps:
- [ ] Confirm `biscuit-terminal/lib/tests/` exists (`ls biscuit-terminal/lib/tests`). It does (e.g. `prelude_exports.rs` is there).
- [ ] Create `biscuit-terminal/lib/tests/html_page_example.rs` with this content:
  ```rust
  //! Representative example: assemble several components into one HTML page.
  //!
  //! This integration test doubles as the canonical "build an HTML page from
  //! components" example required by Project 3 of the renderable kickoff.

  use biscuit_terminal::components::horizontal_rule::HorizontalRule;
  use renderable::browser::BrowserRenderable;
  use renderable::html::HtmlPage;

  /// Compose a `HorizontalRule` fragment into an `HtmlPage` and render it.
  #[test]
  fn components_compose_into_an_html_page() {
      let rule = HorizontalRule::new();
      let fragment = rule.render_html_fragment();

      let page = HtmlPage::from(fragment);
      let html = page.render();

      assert!(html.contains("<html"), "rendered page must have an <html> root");
      assert!(html.contains("<head"), "rendered page must have a <head>");
      assert!(html.contains("<body"), "rendered page must have a <body>");
      assert!(html.contains("<svg"), "the HorizontalRule SVG must appear in the body");
  }
  ```
  (Verify the import paths against the actual module tree: `HorizontalRule`'s public path in `biscuit-terminal/lib/src/lib.rs` or its prelude; `BrowserRenderable`'s path — re-exported at `renderable::browser::BrowserRenderable` from `renderable/src/browser/mod.rs`; `HtmlPage`'s path in `renderable/src/html/mod.rs`. Adjust the `use` lines to whatever those files actually expose. If `HtmlPage::render()` is named differently, use the real name.)
- [ ] Run `cargo nextest run -p biscuit-terminal --test html_page_example` (expected: FAIL — either a compile error if `HtmlPage::render()` is unimplemented, or an assertion failure if `render()` returns `todo!()`/empty). Record the exact failure.

### Task 5.2: Make the example pass

`HtmlPage::render()` and `BrowserFragment<Ready>::render()` may still be `todo!()` after Projects 1–2. If so, implement them minimally — that implementation is in scope for Project 3 because the example cannot exist without it.

**Files:**
- Modify: `renderable/src/html/mod.rs` (`HtmlPage::render`)
- Modify: `renderable/src/browser/fragment.rs` (`BrowserFragment<Ready>::render`)

Steps:
- [ ] Read `renderable/src/browser/fragment.rs` `BrowserFragment<Ready>::render`. If it is `todo!()`, implement it: walk `self.node`, emitting each `ComposableNode` variant — `BlockTag` opens `<tag class="…">`, recursively emits children, closes `</tag>`; `VoidTag` emits `<tag …>`; `TextFragment` emits the string **escaped** via `renderable`'s `escape_text` (from `browser_utils` — confirm the function name); `RawHtml` emits the string **verbatim, unescaped**; `Component` emits the nested `BrowserFragment<Ready>::render()` result. Attribute values are escaped via `escape_attribute`. Keep it minimal — well-formed HTML, no pretty-printing.
- [ ] Read `renderable/src/html/mod.rs` `HtmlPage::render`. If it is `todo!()`, implement it per `rendering-to-a-browser.md` § Render Pipeline / `<head>` Ordering: emit `<!DOCTYPE html><html><head>` then `<meta charset="utf-8">`, `<meta name="viewport" …>`, `<title>…</title>` (from microdata `Title`, else first `<h1>` text, else empty), microdata `<meta>` tags, deduplicated `<link>` tags (page-level then fragment-level, keyed `(rel, href)`), `<style>` blocks (`:root` from `css_variables`, then page `stylesheet`, then component stylesheets), `<script>` blocks, `</head><body>`, each fragment's `render()` output, `</body></html>`. Keep it minimal but complete enough for the Task 5.1 assertions; do not gold-plate metadata fan-out beyond what is needed.
- [ ] Run `cargo build -p renderable` (expected: compiles cleanly).
- [ ] Run `cargo nextest run -p biscuit-terminal --test html_page_example` (expected: `components_compose_into_an_html_page` PASSES).
- [ ] Run `cargo nextest run -p renderable -p biscuit-terminal -p darkmatter` (expected: all tests pass).
- [ ] Commit: `git add -A && git commit -m "feat(biscuit-terminal): add HTML-page composition example"`

---

## Phase 6: Strong Test Coverage

`spec.md` Project 3 requires strong coverage of the migrated surface.

### Task 6.1: Add fragment-composition and stylesheet-rename coverage

**Files:**
- Test: `renderable/src/browser/fragment.rs` (`#[cfg(test)]` module)
- Test: `renderable/src/browser/mod.rs` (`#[cfg(test)]` module)

Steps:
- [ ] In `renderable/src/browser/fragment.rs` `#[cfg(test)]` module, add a test that a `RawHtml` node is emitted unescaped while a `TextFragment` is escaped:
  ```rust
  #[test]
  fn raw_html_is_unescaped_text_fragment_is_escaped() {
      let raw = BrowserFragment::new()
          .define_as_raw_html("<b>hi</b>")
          .finalize()
          .render();
      assert!(raw.contains("<b>hi</b>"), "RawHtml must pass through verbatim");

      let text = BrowserFragment::new()
          .define_as_text_fragment("<b>hi</b>")
          .finalize()
          .render();
      assert!(text.contains("&lt;b&gt;"), "TextFragment must be escaped");
  }
  ```
  (Adjust to the real escaping output of `escape_text` — confirm whether it emits `&lt;`. If `define_as_raw_html` lives in a different state than `Shape`, match the actual builder chain.)
- [ ] In `renderable/src/browser/mod.rs` `#[cfg(test)]` module, add coverage that the renamed collection works end to end:
  ```rust
  #[test]
  fn stylesheet_collection_holds_css_rules() {
      let mut sheet = Stylesheet::new();
      sheet.push(".a", CssStyle::new());
      sheet.push(".b", CssStyle::new());
      let entries = sheet.entries();
      assert_eq!(entries.len(), 2);
      assert_eq!(entries[0].selector, ".a");
      assert_eq!(entries[1].selector, ".b");
  }
  ```
  (Use the real `CssStyle` constructor.)
- [ ] Run `cargo nextest run -p renderable` (expected: all tests pass, including the two new ones).
- [ ] Commit: `git add -A && git commit -m "test(renderable): cover RawHtml escaping and the renamed Stylesheet collection"`

### Task 6.2: Add render_html_page coverage for a migrated component

**Files:**
- Test: `biscuit-terminal/lib/tests/html_page_example.rs` (extend the Phase 5 file)

Steps:
- [ ] Append to `biscuit-terminal/lib/tests/html_page_example.rs`:
  ```rust
  /// `render_html_page` promotes a single component to a standalone page.
  #[test]
  fn render_html_page_promotes_a_single_component() {
      let rule = HorizontalRule::new();
      let page = rule.render_html_page(None);
      let html = page.render();
      assert!(html.contains("<html"), "promoted page must have an <html> root");
      assert!(html.contains("<svg"), "promoted page must contain the rule SVG");
  }
  ```
- [ ] Run `cargo nextest run -p biscuit-terminal --test html_page_example` (expected: both tests PASS).
- [ ] Run `cargo build -p biscuit-terminal` (expected: compiles cleanly).
- [ ] Commit: `git add -A && git commit -m "test(biscuit-terminal): cover render_html_page single-component promotion"`

---

## Verification

Run each check; all must pass before Project 3 is considered complete.

- [ ] **Deprecated trait methods are gone.** `grep -rn "render_to_browser_with_inline_variables" renderable biscuit-terminal darkmatter --include="*.rs"` returns zero hits. `grep -rn "fn render_to_browser" renderable biscuit-terminal darkmatter --include="*.rs"` returns only the inherent `DarkmatterPage::render_to_browser(&self, md: &Markdown)` in `darkmatter/lib/src/layout/page.rs` — no trait method.
- [ ] **The `BrowserRenderable` trait carries exactly two (or three with `as_any`) methods:** `render_html_fragment` and `render_html_page`. Confirm by reading `renderable/src/browser/renderable.rs`.
- [ ] **Stylesheet renames are complete.** `grep -rn "HtmlStyleSheet" renderable biscuit-terminal darkmatter --include="*.rs"` returns zero hits. The declaration block is `CssStyle`, the `(selector, block)` pair is `CssRule`, the collection is `Stylesheet`. `ClassDefinition` and `LinkRel::Stylesheet` / `RelAttribute::Stylesheet` are untouched.
- [ ] **`DarkmatterPage` no longer implements `BrowserRenderable`.** `grep -rn "impl BrowserRenderable for DarkmatterPage" darkmatter --include="*.rs"` returns zero hits.
- [ ] **All three target crates build:** `cargo build -p renderable -p biscuit-terminal -p darkmatter` succeeds with no errors or warnings about unresolved imports.
- [ ] **All tests pass:** `cargo nextest run -p renderable -p biscuit-terminal -p darkmatter` (fallback `cargo test -p renderable && cargo test -p biscuit-terminal && cargo test -p darkmatter`) reports zero failures.
- [ ] **The HTML-page example runs:** `cargo nextest run -p biscuit-terminal --test html_page_example` passes both `components_compose_into_an_html_page` and `render_html_page_promotes_a_single_component`.

# Project 2: Building the New API Surface — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** With the stable base from Project 1 in place, build out the symbols that meet `renderable`'s current and near-future needs: introduce the `MarkdownRenderable` and `AstRenderable` traits, reconcile `BrowserRenderable` to its four-method coexistence shape, wire `ComposableNode::RawHtml` and `Component(BrowserFragment<Ready>)` for structural composition, ship the semantic CSS-variable token layer, reshape `PageOptions`, and give `BrowserFragment` / `HtmlPage` working `render()` implementations.

**Architecture:** `renderable` is a leaf crate (no dependency on `biscuit-terminal` or `darkmatter`). Project 2 only *adds* to the public surface of `renderable` and ships default implementations for the new `BrowserRenderable` methods so existing implementors are not burdened. The deprecated string-producing methods (`render_to_browser`, `render_to_browser_with_inline_variables`) survive Project 2 and are removed in Project 3. Composition is structural per `decisions.md` item 1: `ComposableNode::Component` holds an eager `BrowserFragment<Ready>`, never a boxed trait object. Escaping happens on emit per `decisions.md` item 4 — `TextFragment` is escaped, `RawHtml` is never escaped.

**Tech Stack:** Rust, Cargo workspace (rusty-biscuit monorepo), edition 2024.

**Prerequisites:** Project 1 complete. After Project 1, `renderable` owns the legacy two-method `BrowserRenderable` trait (`renderable::browser::BrowserRenderable`), the `Stylesheet` collection / `CssStyle` / `CssRule` types (`renderable::stylesheet`), and `Layout` + `Color` (`renderable::layout`, `renderable::color`). If a referenced symbol is absent, Project 1 is incomplete — stop and resolve that first.

---

## Authority and conflict resolution

[`decisions.md`](./decisions.md) is the authoritative source for every design choice. Where `spec.md`, `rendering-to-a-browser.md`, `composition-proposal.md`, or `fragment-typestate-design.md` conflict with `decisions.md`, `decisions.md` wins. The decision items most load-bearing for this project: items 1, 2, 3, 4, 5, 7, 8, 9, 10, 11.

## Ordering constraints (read before starting)

1. **Phase A → B → C → D → E** is a hard sequence. Each phase lands as its own commit and must leave `renderable` compiling and testing green.
2. Phase B (composition primitives) depends on Phase A's typestate edits being landed because `ComposableNode` changes shape.
3. Phase C (semantic tokens) is independent of Phase B but must land before Phase D (`PageOptions` reshape) because `PageOptions` references the semantic token enums.
4. Phase E (`render()` bodies) depends on every prior phase — it consumes the final `ComposableNode`, `CssStyle`/`Stylesheet`, semantic-token, and `PageOptions` shapes.
5. NEVER run `cargo build` at the repo root. Always scope with `-p renderable`.
6. Test runner is `cargo nextest run -p renderable`; if `cargo-nextest` is unavailable, fall back to `cargo test -p renderable`.
7. Conventional commits, present-tense, no `Co-Authored-By` trailer. US English (`en-US`) for all symbol names and documentation.

## Pre-flight reconciliation note

The source tree has drifted from `decisions.md` naming. Before Phase E begins, two facts must hold (they are Project 1 / Project 3 territory but Project 2 must not regress them):

- `renderable/src/browser/mod.rs` currently names the collection type `HtmlStyleSheet`. Project 3 Scheme A renames it to `Stylesheet`. **Project 2 does not perform that rename** — it uses whatever name Project 1 left in place. Every code block below that needs the collection type uses the placeholder name `Stylesheet` (the Scheme A name); if the file still says `HtmlStyleSheet` when you reach that step, substitute `HtmlStyleSheet` consistently and leave the rename to Project 3.
- `renderable/src/html/mod.rs` imports `CodeFeature` which does not exist (the type is `PageFeature`). Phase D fixes this as part of the `HtmlPage` rework.

---

## Phase A — Reconcile `BrowserRenderable` to the four-method shape and add the sibling render traits

Project 1 left `renderable::browser::BrowserRenderable` as the legacy two-method trait (`render_to_browser`, `render_to_browser_with_inline_variables`, `as_any`). `decisions.md` item 2A requires the Project 2 coexistence shape: four methods, with the new pair carrying default implementations so existing implementors compile untouched. This phase also adds the `MarkdownRenderable` and `AstRenderable` traits called for by `spec.md` §2.

### Task A1 — Reconcile the `BrowserRenderable` trait

**Files:**
- Modify: `renderable/src/browser/renderable.rs`

- [ ] Open `renderable/src/browser/renderable.rs`. It currently holds the legacy two-method trait moved by Project 1. Replace the entire file contents with the four-method coexistence shape:

  ```rust
  use std::any::Any;
  use std::collections::HashMap;

  use crate::browser::PageOptions;
  use crate::browser::fragment::{BrowserFragment, Ready};
  use crate::html::HtmlPage;

  /// A component capable of rendering itself for browser display.
  ///
  /// During the Project 2 coexistence window this trait carries four
  /// methods. The first two are the legacy string-producing surface,
  /// deprecated and removed in Project 3. The last two are the new
  /// structural surface and ship with default implementations so existing
  /// implementors are not burdened until they choose to migrate.
  ///
  /// ## Notes
  ///
  /// - `render_html_fragment` returns the typestate
  ///   [`BrowserFragment<Ready>`] — the universal "done" currency for
  ///   composition (see decisions.md item 1).
  /// - `render_html_page` returns an [`HtmlPage`], not a `String`; the
  ///   caller then calls [`HtmlPage::render`] for the final string (see
  ///   decisions.md item 7).
  /// - The default `render_html_fragment` wraps the legacy
  ///   `render_to_browser()` output in a [`ComposableNode::RawHtml`]
  ///   fragment, matching the one-line migration shim in decisions.md
  ///   item 12B.
  pub trait BrowserRenderable: std::fmt::Debug + Any {
      /// Renders the component to browser-compatible HTML/SVG.
      ///
      /// Deprecated — removed in Project 3. New code implements
      /// [`render_html_fragment`](BrowserRenderable::render_html_fragment).
      fn render_to_browser(&self) -> String;

      /// Renders the component with inline CSS-variable substitution.
      ///
      /// Deprecated — removed in Project 3. The default ignores
      /// `variables` and calls [`render_to_browser`](BrowserRenderable::render_to_browser).
      fn render_to_browser_with_inline_variables(
          &self,
          _variables: &HashMap<String, String>,
      ) -> String {
          self.render_to_browser()
      }

      /// Produces a fully-composed [`BrowserFragment<Ready>`] for this
      /// component.
      ///
      /// The default implementation wraps `render_to_browser()` output as
      /// caller-owned raw HTML. Components migrate by overriding this with
      /// a typed-node build.
      fn render_html_fragment(&self) -> BrowserFragment<Ready> {
          BrowserFragment::new()
              .define_as_raw_html(self.render_to_browser())
              .finalize()
      }

      /// Promotes this single component to a standalone [`HtmlPage`].
      ///
      /// The default builds an `HtmlPage` from this component's fragment
      /// and applies `page` when supplied.
      fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
          let mut html_page = HtmlPage::from(self.render_html_fragment());
          if let Some(options) = page {
              html_page.apply_page_options(options);
          }
          html_page
      }

      fn as_any(&self) -> &dyn Any;
  }
  ```

- [ ] Note this file now references `PageOptions`, `HtmlPage`, `HtmlPage::from(BrowserFragment<Ready>)`, `HtmlPage::apply_page_options`, and `BrowserFragment::define_as_raw_html`. None of those exist yet in their final shape — they are produced by Phases B and D. This file therefore will **not compile** until Phase D completes. That is expected and intentional: Phase A's commit at Task A4 stages this file but the phase exit criterion below covers it.

### Task A2 — Add the `MarkdownRenderable` trait

**Files:**
- Modify: `renderable/src/markdown.rs`

- [ ] `renderable/src/markdown.rs` is currently empty. Write the `MarkdownRenderable` trait into it:

  ```rust
  use crate::stylesheet::Stylesheet;

  /// A component capable of rendering itself as Markdown output.
  ///
  /// Markdown is a superset of HTML: components that lower cleanly to
  /// ergonomic Markdown implement [`render_markdown`](MarkdownRenderable::render_markdown);
  /// components that need richer styling can consume a [`Stylesheet`] via
  /// [`render_markdown_with_style`](MarkdownRenderable::render_markdown_with_style)
  /// and project the Markdown-addressable rules into the output.
  ///
  /// ## Notes
  ///
  /// - `render_markdown_with_style` defaults to ignoring the stylesheet
  ///   and delegating to `render_markdown`, so a component opts into
  ///   style-aware Markdown only when it has something to do with it.
  pub trait MarkdownRenderable {
      /// Renders the component as a Markdown string.
      fn render_markdown(&self) -> String;

      /// Renders the component as Markdown, optionally consuming a
      /// [`Stylesheet`] for style-aware output.
      ///
      /// The default ignores `style` and delegates to
      /// [`render_markdown`](MarkdownRenderable::render_markdown).
      fn render_markdown_with_style(&self, _style: Option<Stylesheet>) -> String {
          self.render_markdown()
      }
  }
  ```

- [ ] If Project 1 left the collection type named `HtmlStyleSheet` rather than `Stylesheet`, substitute `HtmlStyleSheet` for `Stylesheet` in both the `use` line and the method signature. Confirm the actual name with `grep -rn "pub struct \(Html\)\?Stylesheet\|pub struct HtmlStyleSheet" renderable/src` before writing.

### Task A3 — Add the `AstRenderable` trait

**Files:**
- Modify: `renderable/src/ast.rs`

- [ ] `renderable/src/ast.rs` is currently empty. Write a minimal `AstRenderable` trait. The AST node model is out of scope for this project, so the trait renders to the existing `RenderTarget`-adjacent string form and is deliberately thin:

  ```rust
  /// A component capable of rendering itself to an abstract syntax tree
  /// representation.
  ///
  /// The AST node model is intentionally not specified yet — this trait
  /// exists so AST becomes a first-class render target alongside
  /// `TerminalRenderable`, `BrowserRenderable`, and `MarkdownRenderable`.
  /// The single method returns a serialized AST string; a typed node
  /// model is deferred until a concrete consumer needs it.
  ///
  /// ## Notes
  ///
  /// - This is a placeholder surface. It will gain a typed node return
  ///   value in a future project once the AST representation is designed.
  pub trait AstRenderable {
      /// Renders the component to a serialized AST string.
      fn render_ast(&self) -> String;
  }
  ```

### Task A4 — Stage and commit Phase A

**Files:** none (verification only)

- [ ] Do **not** run `cargo build` yet — `renderable/src/browser/renderable.rs` references symbols produced by later phases and will not compile. This is the only phase whose commit lands a temporarily-non-compiling file; phases B–E restore green.
- [ ] Stage and commit:

  ```bash
  git add renderable/src/browser/renderable.rs renderable/src/markdown.rs renderable/src/ast.rs
  git commit -m "feat(renderable): reconcile BrowserRenderable and add Markdown/Ast render traits"
  ```

  Expected: commit succeeds. No `Co-Authored-By` trailer.

---

## Phase B — Composition primitives: `RawHtml`, eager `Component`, and `define_as_raw_html`

`decisions.md` item 1 makes composition structural — `ComposableNode::Component` holds a fully-rendered `BrowserFragment<Ready>`, never a `Box<dyn BrowserRenderable>`. Item 2B adds the `RawHtml(String)` escape hatch and its `define_as_raw_html` builder. This phase reshapes `ComposableNode` and adds the builders.

### Task B1 — Reshape `ComposableNode`

**Files:**
- Modify: `renderable/src/browser/fragment.rs`

- [ ] In `renderable/src/browser/fragment.rs`, replace the `ComposableNode` enum (currently lines 69–74) with the four-variant structural form:

  ```rust
  /// A composable node extends `HtmlNode` with two extra shapes: caller-owned
  /// raw HTML, and nested components.
  ///
  /// Per decisions.md item 1, the [`Component`](ComposableNode::Component)
  /// variant holds an eager [`BrowserFragment<Ready>`] — composition is
  /// structural, not a boxed trait object. This makes the node tree
  /// homogeneous: every nested component is already "done", so page-level
  /// aggregation is a pure recursive walk with no trait calls.
  pub enum ComposableNode {
      /// A non-void element with attributes and children.
      BlockTag(HtmlBlockTag),
      /// A void element with attributes and no children.
      VoidTag(HtmlVoidTag),
      /// A run of literal text. **Escaped on emit** by the renderer.
      TextFragment(String),
      /// Caller-owned prebuilt HTML (SVG, third-party markup). **Never
      /// escaped** by the renderer — the caller owns correctness.
      RawHtml(String),
      /// A nested, fully-rendered component fragment.
      Component(Box<BrowserFragment<Ready>>),
  }
  ```

- [ ] In the same file, the legacy two-method `BrowserRenderable` placeholder trait was deleted by Project 1 and the trait now lives in `renderable/src/browser/renderable.rs`. Confirm there is no `pub trait BrowserRenderable` declaration remaining inside `fragment.rs` (Project 1 removed it). If one remains, delete it — the canonical trait is in `renderable.rs`.

- [ ] Confirm the `use crate::{…}` block at the top of `fragment.rs` no longer imports `BrowserRenderable` (the `Component` variant no longer needs the trait — it holds a concrete `BrowserFragment<Ready>`). If `BrowserRenderable` appears in the `use` block, remove it. The block should read:

  ```rust
  use std::collections::HashMap;
  use std::marker::PhantomData;

  use crate::{
      browser::{ComponentStylesheet, feature::PageFeature},
      html::tag::{BlockTag, HtmlAttribute, HtmlBlockTag, HtmlVoidTag, VoidTag, link::LinkTag},
      microdata::MicrodataKey,
  };
  ```

### Task B2 — Add the `define_as_raw_html` builder

**Files:**
- Modify: `renderable/src/browser/fragment.rs`

- [ ] The typestate machine has no state for raw HTML. `RawHtml`, like `TextFragment`, has no attributes and no children, so it routes through the existing `RefineText` state — the cross-cutting builders (stylesheet, features, metadata, dependency links) still apply, and `finalize()` is the only state-changing method. In `renderable/src/browser/fragment.rs`, inside `impl BrowserFragment<Shape>`, add a fourth `define_as_*` builder immediately after `define_as_text_fragment`:

  ```rust
      /// Commit the fragment to a raw-HTML shape. The string is caller-owned
      /// and **never escaped** by the renderer. Transitions to [`RefineText`]
      /// so the cross-cutting builders still apply.
      ///
      /// Use this only for final, already-escaped markup — SVG, third-party
      /// HTML. For literal text content use
      /// [`define_as_text_fragment`](BrowserFragment::define_as_text_fragment),
      /// which is escaped on emit.
      pub fn define_as_raw_html(self, html: impl Into<String>) -> BrowserFragment<RefineText> {
          self.into_state(Some(ComposableNode::RawHtml(html.into())))
      }
  ```

- [ ] Update the `BrowserFragment` struct's transition doc comment (the `text` diagram around line 92–98) so it lists the raw-HTML entry point. Change the diagram to:

  ```text
  /// Shape ──define_as_block_tag──▶ RefineBlock ───┐
  ///       ──define_as_void_tag──▶ RefineVoid ─────┼── finalize() ──▶ Ready
  ///       ──define_as_text_fragment──▶ RefineText ┤
  ///       ──define_as_raw_html──▶ RefineText ─────┘
  ```

### Task B3 — Allow nested components as children

**Files:**
- Modify: `renderable/src/browser/fragment.rs`

- [ ] `BrowserFragment<RefineBlock>::add_child` already accepts a `ComposableNode`, so `add_child(ComposableNode::Component(child_fragment))` works once `ComposableNode::Component` holds `BrowserFragment<Ready>`. Add a convenience builder so callers do not hand-construct the variant. Inside `impl BrowserFragment<RefineBlock>`, after `add_child`, add:

  ```rust
      /// Append a fully-rendered child component fragment.
      ///
      /// Convenience over `add_child(ComposableNode::Component(child))` —
      /// the recursion point that lets components compose other components.
      pub fn add_component(self, child: BrowserFragment<Ready>) -> Self {
          self.add_child(ComposableNode::Component(child))
      }
  ```

### Task B4 — Unit tests for composition primitives

**Files:**
- Modify: `renderable/src/browser/fragment.rs` (add a `#[cfg(test)]` module if absent)

- [ ] At the bottom of `renderable/src/browser/fragment.rs`, add a test module covering the new builders. These tests assert state transitions and node identity, not rendered output (rendering is Phase E):

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::html::tag::BlockTag;

      #[test]
      fn define_as_raw_html_finalizes_to_ready() {
          let fragment = BrowserFragment::new()
              .define_as_raw_html("<svg></svg>")
              .finalize();
          match fragment.node {
              Some(ComposableNode::RawHtml(ref html)) => assert_eq!(html, "<svg></svg>"),
              _ => panic!("expected RawHtml node"),
          }
      }

      #[test]
      fn add_component_nests_a_ready_fragment() {
          let child = BrowserFragment::new()
              .define_as_text_fragment("child")
              .finalize();
          let parent = BrowserFragment::new()
              .define_as_block_tag(BlockTag::Div, "parent")
              .add_component(child)
              .finalize();
          match parent.node {
              Some(ComposableNode::BlockTag(ref block)) => {
                  assert_eq!(block.content.children.len(), 1);
                  assert!(matches!(
                      block.content.children[0],
                      ComposableNode::Component(_)
                  ));
              }
              _ => panic!("expected BlockTag node"),
          }
      }

      #[test]
      fn raw_html_carries_cross_cutting_builders() {
          let fragment = BrowserFragment::new()
              .define_as_raw_html("<svg></svg>")
              .add_metadata_keypair(MicrodataKey::Title, "Diagram")
              .finalize();
          assert_eq!(
              fragment.metadata.get(&MicrodataKey::Title).map(String::as_str),
              Some("Diagram")
          );
      }
  }
  ```

### Task B5 — Verify and commit Phase B

**Files:** none (verification only)

- [ ] `renderable` still will not compile because `renderable.rs` from Phase A references not-yet-built `PageOptions` / `HtmlPage` machinery. Do **not** expect a clean build yet. Confirm the *fragment.rs* edits are syntactically isolated by reviewing them — the phase exit is the commit.
- [ ] Stage and commit:

  ```bash
  git add renderable/src/browser/fragment.rs
  git commit -m "feat(renderable): add RawHtml node and structural component composition"
  ```

  Expected: commit succeeds. No `Co-Authored-By` trailer.

---

## Phase C — Semantic CSS-variable token layer

`decisions.md` item 3 ships **both** a palette layer (Tailwind-derived, open set, handled by `Color::Var`) and a curated semantic layer. The semantic layer is compiler-checked: typed enums `SemanticToken`, `SpaceToken`, `FontToken` are the single source of truth and double as the generator for the `:root` defaults. Three families ship now: colors, spacing, typography (item 3B). Naming is Tailwind-style namespaced (item 3C).

### Task C1 — Create the token module

**Files:**
- Create: `renderable/src/tokens.rs`
- Modify: `renderable/src/lib.rs`

- [ ] In `renderable/src/lib.rs`, add a module declaration. After the existing `pub mod target;` line add:

  ```rust
  pub mod tokens;
  ```

- [ ] Create `renderable/src/tokens.rs` with the three typed token enums. Each enum knows its `--name`, its default CSS value, and a `var()` reference form. The `ALL` arrays and `root_defaults()` make the enums the generator for the page-level `:root` block:

  ```rust
  //! Compiler-checked semantic CSS-variable tokens.
  //!
  //! Three families ship: colors, spacing, typography. Each token is a
  //! typed enum variant that knows its `--name`, its default value, and
  //! how to reference itself as `var(--name)`. The page declares these in
  //! its `:root` block; components consume them via [`SemanticToken::var`]
  //! and friends. Components must reference this semantic layer, not the
  //! open palette layer (`Color::Var`).

  /// A curated semantic color token. Defaults reference the palette layer
  /// so a caller re-themes by overriding the semantic token, not the
  /// palette.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum SemanticToken {
      /// Page background color.
      Bg,
      /// Primary foreground / text color.
      Fg,
      /// Muted / secondary foreground color.
      FgMuted,
      /// Accent color for interactive elements.
      Accent,
      /// Error / danger color.
      Error,
      /// Warning color.
      Warning,
      /// Success color.
      Success,
      /// Default border color.
      Border,
  }

  impl SemanticToken {
      /// Every semantic color token, in declaration order.
      pub const ALL: [SemanticToken; 8] = [
          SemanticToken::Bg,
          SemanticToken::Fg,
          SemanticToken::FgMuted,
          SemanticToken::Accent,
          SemanticToken::Error,
          SemanticToken::Warning,
          SemanticToken::Success,
          SemanticToken::Border,
      ];

      /// The custom-property name, without the leading `--`.
      pub fn name(&self) -> &'static str {
          match self {
              SemanticToken::Bg => "color-bg",
              SemanticToken::Fg => "color-fg",
              SemanticToken::FgMuted => "color-fg-muted",
              SemanticToken::Accent => "color-accent",
              SemanticToken::Error => "color-error",
              SemanticToken::Warning => "color-warning",
              SemanticToken::Success => "color-success",
              SemanticToken::Border => "color-border",
          }
      }

      /// The default CSS value for this token. Defaults reference Tailwind
      /// palette hex values directly so the semantic layer is self-contained.
      pub fn default_value(&self) -> &'static str {
          match self {
              SemanticToken::Bg => "#ffffff",
              SemanticToken::Fg => "#1f2937",
              SemanticToken::FgMuted => "#6b7280",
              SemanticToken::Accent => "#3b82f6",
              SemanticToken::Error => "#ef4444",
              SemanticToken::Warning => "#f59e0b",
              SemanticToken::Success => "#22c55e",
              SemanticToken::Border => "#e5e7eb",
          }
      }

      /// The `var(--name)` reference form for use in component CSS.
      pub fn var(&self) -> String {
          format!("var(--{})", self.name())
      }
  }

  /// A spacing-scale token. Values follow a Tailwind-style rem scale.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum SpaceToken {
      /// `0.25rem`
      One,
      /// `0.5rem`
      Two,
      /// `0.75rem`
      Three,
      /// `1rem`
      Four,
      /// `1.5rem`
      Six,
      /// `2rem`
      Eight,
  }

  impl SpaceToken {
      /// Every spacing token, in scale order.
      pub const ALL: [SpaceToken; 6] = [
          SpaceToken::One,
          SpaceToken::Two,
          SpaceToken::Three,
          SpaceToken::Four,
          SpaceToken::Six,
          SpaceToken::Eight,
      ];

      /// The custom-property name, without the leading `--`.
      pub fn name(&self) -> &'static str {
          match self {
              SpaceToken::One => "space-1",
              SpaceToken::Two => "space-2",
              SpaceToken::Three => "space-3",
              SpaceToken::Four => "space-4",
              SpaceToken::Six => "space-6",
              SpaceToken::Eight => "space-8",
          }
      }

      /// The default CSS length value for this token.
      pub fn default_value(&self) -> &'static str {
          match self {
              SpaceToken::One => "0.25rem",
              SpaceToken::Two => "0.5rem",
              SpaceToken::Three => "0.75rem",
              SpaceToken::Four => "1rem",
              SpaceToken::Six => "1.5rem",
              SpaceToken::Eight => "2rem",
          }
      }

      /// The `var(--name)` reference form for use in component CSS.
      pub fn var(&self) -> String {
          format!("var(--{})", self.name())
      }
  }

  /// A typography token (font family / size).
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum FontToken {
      /// Default sans-serif font stack.
      Sans,
      /// Monospace font stack.
      Mono,
      /// Base body font size.
      SizeBase,
      /// Small font size.
      SizeSm,
      /// Large font size.
      SizeLg,
  }

  impl FontToken {
      /// Every typography token, in declaration order.
      pub const ALL: [FontToken; 5] = [
          FontToken::Sans,
          FontToken::Mono,
          FontToken::SizeBase,
          FontToken::SizeSm,
          FontToken::SizeLg,
      ];

      /// The custom-property name, without the leading `--`.
      pub fn name(&self) -> &'static str {
          match self {
              FontToken::Sans => "font-sans",
              FontToken::Mono => "font-mono",
              FontToken::SizeBase => "font-size-base",
              FontToken::SizeSm => "font-size-sm",
              FontToken::SizeLg => "font-size-lg",
          }
      }

      /// The default CSS value for this token.
      pub fn default_value(&self) -> &'static str {
          match self {
              FontToken::Sans => {
                  "ui-sans-serif, system-ui, -apple-system, \
                   'Segoe UI', sans-serif"
              }
              FontToken::Mono => {
                  "ui-monospace, SFMono-Regular, 'SF Mono', \
                   Menlo, monospace"
              }
              FontToken::SizeBase => "1rem",
              FontToken::SizeSm => "0.875rem",
              FontToken::SizeLg => "1.125rem",
          }
      }

      /// The `var(--name)` reference form for use in component CSS.
      pub fn var(&self) -> String {
          format!("var(--{})", self.name())
      }
  }

  /// Returns the full `(name, value)` list of semantic-layer defaults
  /// across all three token families, in declaration order.
  ///
  /// `HtmlPage` emits these as the page-level `:root { … }` block unless a
  /// caller overrides specific tokens via `PageOptions`. Each pair is
  /// `(custom-property-name-without-leading-dashes, css-value)`.
  pub fn root_defaults() -> Vec<(String, String)> {
      let mut out = Vec::new();
      for token in SemanticToken::ALL {
          out.push((token.name().to_string(), token.default_value().to_string()));
      }
      for token in SpaceToken::ALL {
          out.push((token.name().to_string(), token.default_value().to_string()));
      }
      for token in FontToken::ALL {
          out.push((token.name().to_string(), token.default_value().to_string()));
      }
      out
  }
  ```

### Task C2 — Unit tests for the token layer

**Files:**
- Modify: `renderable/src/tokens.rs`

- [ ] At the bottom of `renderable/src/tokens.rs`, add a test module verifying naming conventions, `var()` form, and the `root_defaults()` aggregate:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn semantic_token_var_form_is_well_formed() {
          assert_eq!(SemanticToken::Bg.var(), "var(--color-bg)");
          assert_eq!(SemanticToken::Error.var(), "var(--color-error)");
      }

      #[test]
      fn space_token_var_form_is_well_formed() {
          assert_eq!(SpaceToken::Two.var(), "var(--space-2)");
      }

      #[test]
      fn font_token_var_form_is_well_formed() {
          assert_eq!(FontToken::Mono.var(), "var(--font-mono)");
      }

      #[test]
      fn every_color_token_name_uses_the_color_prefix() {
          for token in SemanticToken::ALL {
              assert!(
                  token.name().starts_with("color-"),
                  "semantic token {token:?} must use the color- prefix"
              );
          }
      }

      #[test]
      fn root_defaults_covers_every_token() {
          let defaults = root_defaults();
          let expected =
              SemanticToken::ALL.len() + SpaceToken::ALL.len() + FontToken::ALL.len();
          assert_eq!(defaults.len(), expected);
          // Names are unique.
          let mut names: Vec<&str> = defaults.iter().map(|(n, _)| n.as_str()).collect();
          names.sort_unstable();
          let unique = names.len();
          names.dedup();
          assert_eq!(names.len(), unique, "token names must be unique");
      }

      #[test]
      fn root_defaults_values_are_non_empty() {
          for (name, value) in root_defaults() {
              assert!(!name.is_empty(), "token name must not be empty");
              assert!(!value.is_empty(), "token {name} value must not be empty");
          }
      }
  }
  ```

### Task C3 — Verify and commit Phase C

**Files:** none (verification only)

- [ ] `renderable` still does not build as a whole (Phase A's `renderable.rs` is unresolved). Verify the new module in isolation by checking only the token tests will pass once the crate compiles — review the file for syntax. The phase exit is the commit.
- [ ] Stage and commit:

  ```bash
  git add renderable/src/tokens.rs renderable/src/lib.rs
  git commit -m "feat(renderable): add compiler-checked semantic CSS token layer"
  ```

  Expected: commit succeeds. No `Co-Authored-By` trailer.

---

## Phase D — Reshape `PageOptions` and `HtmlPage`

`decisions.md` item 7 reshapes `PageOptions` (drops `Layout`, adds external-asset paths), removes the standalone `RenderOptions`, and splits `HtmlPage` into a page model plus three pure output methods (`render`, `stylesheet`, `inline_code`). Item 8 removes `HtmlPage::title` in favor of a `set_title()` method writing the `Title` microdata key. Item 6 confirms `Layout` is not wired into `PageOptions`. This phase reshapes the structs and stubs `render()` (full body lands in Phase E).

### Task D1 — Reshape `PageOptions`

**Files:**
- Modify: `renderable/src/browser/mod.rs`

- [ ] In `renderable/src/browser/mod.rs`, replace the `PageOptions` struct (currently lines 90–100, carrying a `layout` field) with the item-7 shape. Per item 6, drop `layout`; per item 7, add the two external-asset path fields:

  ```rust
  use std::path::PathBuf;

  /// Caller-supplied options that shape page assembly and rendering.
  ///
  /// Per decisions.md item 6, `PageOptions` carries no `Layout`: page
  /// background, margins, and padding are expressed through the page
  /// [`Stylesheet`] (rulesets on `html` / `body`). Per item 7, the
  /// inline-vs-external asset choice lives here — `None` means inline.
  #[derive(Default)]
  pub struct PageOptions {
      /// Page-level stylesheet. Wins over component defaults at equal
      /// specificity. `None` leaves the page with only component styles.
      pub stylesheet: Option<Stylesheet>,
      /// Ordered `(variable_name, value)` overrides for the page-level
      /// `:root` block. A name present here replaces the semantic-token
      /// default; names not listed keep their default.
      pub css_variables: Option<Vec<(String, String)>>,
      /// When `Some`, the page's rolled-up CSS is emitted as an external
      /// `<link href="…">` instead of an inline `<style>`. The path is
      /// enforced relative so the `href` stays portable.
      pub external_stylesheet: Option<PathBuf>,
      /// When `Some`, the page's rolled-up JS is emitted as an external
      /// `<script src="…">` instead of an inline `<script>`. The path is
      /// enforced relative so the `src` stays portable.
      pub external_code: Option<PathBuf>,
  }
  ```

- [ ] If Project 1 left the collection type named `HtmlStyleSheet`, the `stylesheet` field type above must read `Option<HtmlStyleSheet>`. Confirm the actual name with `grep -rn "pub struct HtmlStyleSheet\|pub struct Stylesheet" renderable/src` and substitute consistently.

- [ ] In the same file, confirm the `use` block at the top no longer needs `Layout` (item 6 removed it). If `Layout` is imported and unused, remove it from the `use` block. Add `use std::path::PathBuf;` at the top of the file if it is not already shown by the block above being placed inline.

### Task D2 — Rework `HtmlPage`: remove `title`, fix `CodeFeature`, add `apply_page_options` and output methods

**Files:**
- Modify: `renderable/src/html/mod.rs`

- [ ] `renderable/src/html/mod.rs` currently imports a non-existent `CodeFeature` and stores a `title: Option<String>` field that item 8 removes. Replace the whole file with the reworked `HtmlPage`. The struct owns metadata as a `MicrodataMap`-style accumulation, drops `title`, uses `PageFeature` (the real type), and gains `apply_page_options`, `set_title`, and the three pure output methods. `render()` / `stylesheet()` / `inline_code()` bodies are stubbed with `todo!()` and filled in Phase E:

  ```rust
  use std::collections::HashMap;

  use crate::{
      browser::{
          PageOptions, Stylesheet, fragment::BrowserFragment, fragment::Ready,
          feature::PageFeature,
      },
      html::tag::{link::LinkTag, meta::MetaTag},
      microdata::MicrodataKey,
  };

  /// A fully-assembled HTML page: a tree of fragments plus page-level
  /// `<head>` state.
  ///
  /// Per decisions.md item 8, `HtmlPage` does not store a `title` field —
  /// [`set_title`](HtmlPage::set_title) writes the `Title` microdata key so
  /// the title fans out into HTML / OpenGraph / Twitter / Schema.org tags.
  /// `HtmlPage` owns all metadata; component metadata bubbles up the
  /// fragment tree and page-level metadata wins on conflict.
  pub struct HtmlPage {
      /// Page-level stylesheet. Wins over component defaults at equal
      /// specificity.
      stylesheet: Stylesheet,
      /// Page-level `<link>` tags. Deduped against fragment dependency
      /// links at render time.
      links: Vec<LinkTag>,
      /// Page-level inline `<script>` blocks.
      script_blocks: Vec<String>,
      /// Page-level `<meta>` tags not derived from microdata.
      meta: Vec<MetaTag>,
      /// Page-level features rolled up from fragments and the caller.
      features: Vec<PageFeature>,
      /// Page-level microdata. Page entries win over component entries on
      /// key conflict (decisions.md item 8).
      metadata: HashMap<MicrodataKey, String>,
      /// Owned fragments. A page is the natural lifetime root for the
      /// fragments it composes.
      fragments: Vec<BrowserFragment<Ready>>,
      /// `(variable_name, value)` overrides for the `:root` block. `None`
      /// means the page emits only semantic-token defaults.
      css_variables: Option<Vec<(String, String)>>,
      /// Inline-vs-external CSS choice. `Some(path)` → external `<link>`.
      external_stylesheet: Option<std::path::PathBuf>,
      /// Inline-vs-external JS choice. `Some(path)` → external `<script src>`.
      external_code: Option<std::path::PathBuf>,
  }

  impl Default for HtmlPage {
      fn default() -> HtmlPage {
          HtmlPage {
              stylesheet: Stylesheet::new(),
              links: Vec::new(),
              script_blocks: Vec::new(),
              meta: Vec::new(),
              features: Vec::new(),
              metadata: HashMap::new(),
              fragments: Vec::new(),
              css_variables: None,
              external_stylesheet: None,
              external_code: None,
          }
      }
  }

  impl From<BrowserFragment<Ready>> for HtmlPage {
      fn from(fragment: BrowserFragment<Ready>) -> HtmlPage {
          HtmlPage {
              fragments: vec![fragment],
              ..HtmlPage::default()
          }
      }
  }

  impl HtmlPage {
      /// Construct a page from an ordered list of fragments.
      pub fn from_fragments(fragments: Vec<BrowserFragment<Ready>>) -> HtmlPage {
          HtmlPage {
              fragments,
              ..HtmlPage::default()
          }
      }

      /// Append a fragment to the page body.
      pub fn add_fragment(&mut self, fragment: BrowserFragment<Ready>) -> &mut HtmlPage {
          self.fragments.push(fragment);
          self
      }

      /// Set the page title.
      ///
      /// Per decisions.md item 8 this writes the `Title` microdata key so
      /// the title fans out into HTML / OpenGraph / Twitter / Schema.org
      /// tags — one code path, no dedicated `title` field.
      pub fn set_title(&mut self, title: impl Into<String>) -> &mut HtmlPage {
          self.metadata.insert(MicrodataKey::Title, title.into());
          self
      }

      /// Add a page-level microdata key/value pair. Page-level entries win
      /// over component entries on key conflict.
      pub fn add_metadata(&mut self, key: MicrodataKey, value: impl Into<String>) -> &mut HtmlPage {
          self.metadata.insert(key, value.into());
          self
      }

      /// Add a `<link>` tag to `<head>`.
      pub fn add_link(&mut self, link: LinkTag) -> &mut HtmlPage {
          self.links.push(link);
          self
      }

      /// Add a page-level inline `<script>` block.
      pub fn add_script_block(&mut self, code_block: impl Into<String>) -> &mut HtmlPage {
          self.script_blocks.push(code_block.into());
          self
      }

      /// Apply a [`PageOptions`] to this page in place.
      ///
      /// Replaces the stylesheet when one is supplied, merges CSS-variable
      /// overrides, and records the inline-vs-external asset choices.
      pub fn apply_page_options(&mut self, options: PageOptions) -> &mut HtmlPage {
          if let Some(stylesheet) = options.stylesheet {
              self.stylesheet = stylesheet;
          }
          if let Some(variables) = options.css_variables {
              self.css_variables = Some(variables);
          }
          self.external_stylesheet = options.external_stylesheet;
          self.external_code = options.external_code;
          self
      }

      /// Dedups the page's own `<link>` tags **and** the dependency links
      /// pulled from every composed fragment, returning the unified list in
      /// first-seen order.
      ///
      /// Identity is [`LinkTag::dedup_key`] — `(rel, href)`. Page-level
      /// links are seen first and win ordering ties.
      pub fn collect_dedup_links(&self) -> Vec<&LinkTag> {
          let mut seen = std::collections::HashSet::new();
          let mut out = Vec::new();
          let mut push_if_new = |link: &'_ LinkTag, out: &mut Vec<&'_ LinkTag>| {
              if seen.insert(link.dedup_key()) {
                  out.push(link);
              }
          };
          for link in &self.links {
              push_if_new(link, &mut out);
          }
          for fragment in &self.fragments {
              for link in fragment.dependency_links() {
                  push_if_new(link, &mut out);
              }
          }
          out
      }

      /// Renders the page to a complete HTML string.
      ///
      /// Pure: never performs I/O. When [`PageOptions`] selected external
      /// assets, this emits `<link>` / `<script src>` references; the
      /// caller pulls content from [`stylesheet`](HtmlPage::stylesheet) /
      /// [`inline_code`](HtmlPage::inline_code) and writes it.
      pub fn render(&self) -> String {
          todo!("Phase E")
      }

      /// Returns the page's rolled-up CSS text (`:root` block + page
      /// stylesheet + component default stylesheets).
      pub fn stylesheet(&self) -> String {
          todo!("Phase E")
      }

      /// Returns the page's rolled-up JS text (page script blocks +
      /// per-fragment feature code).
      pub fn inline_code(&self) -> String {
          todo!("Phase E")
      }
  }
  ```

- [ ] This file references `BrowserFragment::dependency_links()` (an accessor for the private `dependency_links` field) and `Stylesheet::new()`. The accessor is added in Task D3. `Stylesheet::new()` already exists on the collection type (`HtmlStyleSheet::new()` per Project 1) — if the type is still named `HtmlStyleSheet`, substitute `HtmlStyleSheet` for `Stylesheet` throughout this file.

### Task D3 — Add fragment accessors needed by page aggregation

**Files:**
- Modify: `renderable/src/browser/fragment.rs`

- [ ] `HtmlPage` aggregation needs read access to a fragment's private fields. In `renderable/src/browser/fragment.rs`, add a read-only accessor impl block for `BrowserFragment<Ready>` (after the existing `impl BrowserFragment<Ready>` block):

  ```rust
  impl BrowserFragment<Ready> {
      /// The fragment's top-level composable node, if set.
      pub fn node(&self) -> Option<&ComposableNode> {
          self.node.as_ref()
      }

      /// The fragment's component stylesheet, if attached.
      pub fn stylesheet(&self) -> Option<&ComponentStylesheet> {
          self.stylesheet.as_ref()
      }

      /// The page-level features this fragment depends on.
      pub fn features(&self) -> &[PageFeature] {
          &self.features
      }

      /// The microdata key/value pairs this fragment contributes.
      pub fn metadata(&self) -> &HashMap<MicrodataKey, String> {
          &self.metadata
      }

      /// The `<link>` dependencies this fragment declares.
      pub fn dependency_links(&self) -> &[LinkTag] {
          &self.dependency_links
      }
  }
  ```

- [ ] These accessors are on `BrowserFragment<Ready>` only — they read finalized state. The test module added in Task B4 accesses `fragment.node` and `fragment.metadata` as fields directly; that is fine because the tests are in the same module and can see private fields. Leave the tests as written.

### Task D4 — Verify and commit Phase D

**Files:** none (verification only)

- [ ] Run `cargo build -p renderable`. **This is the first phase where the crate must compile cleanly** — Phases A–C deferred compilation because they referenced symbols this phase produces. Expected: `Compiling renderable …` then `Finished` with no errors. Common failure modes and fixes:
  - Unresolved `Stylesheet` — Project 1 named the collection `HtmlStyleSheet`; substitute that name in `renderable.rs`, `markdown.rs`, `browser/mod.rs`, and `html/mod.rs`.
  - `BrowserFragment::define_as_raw_html` not found — confirm Phase B Task B2 landed.
  - `HtmlPage::apply_page_options` / `HtmlPage::from` mismatch — confirm Task D2's `From<BrowserFragment<Ready>>` impl is present.
- [ ] Run `cargo nextest run -p renderable` (fallback `cargo test -p renderable`). Expected: all tests pass, `0 failed` — the Phase B and Phase C test modules now execute.
- [ ] Stage and commit:

  ```bash
  git add renderable/src/browser/mod.rs renderable/src/html/mod.rs renderable/src/browser/fragment.rs
  git commit -m "feat(renderable): reshape PageOptions and HtmlPage for the new API surface"
  ```

  Expected: commit succeeds. No `Co-Authored-By` trailer.

---

## Phase E — Render implementations and browser utilities

This phase fills the `todo!()` bodies: `ComponentStylesheet::as_stylesheet`, `MetaTag::render`, `BrowserFragment::render` / `validate_render_content`, and `HtmlPage::render` / `stylesheet` / `inline_code`. It also ships the `browser::utils` helper module from `browser-utils.md`. `decisions.md` item 4 governs escaping: `TextFragment` escaped, `RawHtml` never escaped, attribute values escaped.

### Task E1 — Build the `browser::utils` helper module

**Files:**
- Modify: `renderable/src/browser/utils.rs`

- [ ] `renderable/src/browser/utils.rs` is currently empty. Implement the escaping and class/style helpers from `browser-utils.md`. The `Color` helper (browser-utils §6) is **omitted** — Project 1 already moved a real `Color` into `renderable::color`, so the temporary placeholder is unnecessary. Write:

  ```rust
  //! Helpers for component authors. Scope is narrow: each helper either
  //! prevents a correctness bug (escaping) or removes boilerplate that
  //! recurs across components.

  use std::borrow::Cow;

  /// Escapes the three characters that change parser state inside element
  /// content: `&`, `<`, `>`.
  ///
  /// Returns the input borrowed when no escaping is needed — the common
  /// case allocates nothing.
  pub fn escape_text(input: &str) -> Cow<'_, str> {
      if input.bytes().any(|b| matches!(b, b'&' | b'<' | b'>')) {
          let mut out = String::with_capacity(input.len() + 8);
          for ch in input.chars() {
              match ch {
                  '&' => out.push_str("&amp;"),
                  '<' => out.push_str("&lt;"),
                  '>' => out.push_str("&gt;"),
                  other => out.push(other),
              }
          }
          Cow::Owned(out)
      } else {
          Cow::Borrowed(input)
      }
  }

  /// Escapes the four characters relevant inside a double-quoted attribute
  /// value: `&`, `<`, `"`, `'`. `>` is intentionally left alone.
  pub fn escape_attribute(input: &str) -> Cow<'_, str> {
      if input.bytes().any(|b| matches!(b, b'&' | b'<' | b'"' | b'\'')) {
          let mut out = String::with_capacity(input.len() + 8);
          for ch in input.chars() {
              match ch {
                  '&' => out.push_str("&amp;"),
                  '<' => out.push_str("&lt;"),
                  '"' => out.push_str("&quot;"),
                  '\'' => out.push_str("&#39;"),
                  other => out.push(other),
              }
          }
          Cow::Owned(out)
      } else {
          Cow::Borrowed(input)
      }
  }

  /// Joins class-name parts with a single space. `None` and parts that are
  /// empty after trimming are dropped. Inner whitespace is preserved.
  pub fn classes<I, S>(parts: I) -> String
  where
      I: IntoIterator<Item = Option<S>>,
      S: AsRef<str>,
  {
      let mut out = String::new();
      for part in parts {
          let Some(part) = part else { continue };
          let trimmed = part.as_ref().trim();
          if trimmed.is_empty() {
              continue;
          }
          if !out.is_empty() {
              out.push(' ');
          }
          out.push_str(trimmed);
      }
      out
  }

  /// Converts an arbitrary string into a CSS-safe identifier segment.
  ///
  /// Rules: lowercase ASCII letters/digits pass through; every other
  /// character becomes `-`; consecutive `-` collapse; leading/trailing `-`
  /// trim; a leading digit gets a `_` prefix; an empty result becomes `_`.
  /// Unicode normalization is intentionally not performed — the rusty
  /// biscuit workspace targets ASCII identifiers.
  pub fn css_slug(input: &str) -> String {
      let mut out = String::with_capacity(input.len());
      let mut last_was_dash = false;
      for ch in input.chars() {
          if ch.is_ascii_alphanumeric() {
              out.push(ch.to_ascii_lowercase());
              last_was_dash = false;
          } else if !last_was_dash && !out.is_empty() {
              out.push('-');
              last_was_dash = true;
          }
      }
      while out.ends_with('-') {
          out.pop();
      }
      if out.is_empty() {
          return "_".to_string();
      }
      if out.starts_with(|c: char| c.is_ascii_digit()) {
          out.insert(0, '_');
      }
      out
  }

  /// Returns a `var(--name)` CSS variable reference. The leading `--` is
  /// added by the helper; pass the bare name.
  pub fn css_var(name: &str) -> String {
      format!("var(--{name})")
  }

  /// Returns a `var(--name, fallback)` CSS variable reference. The fallback
  /// is emitted verbatim — not escaped or quoted.
  pub fn css_var_with_fallback(name: &str, fallback: &str) -> String {
      format!("var(--{name}, {fallback})")
  }

  /// An inline-style builder. Stores declarations as an ordered `Vec` so
  /// declaration order is preserved.
  #[derive(Debug, Default, Clone)]
  pub struct Style {
      declarations: Vec<(String, String)>,
  }

  impl Style {
      /// An empty style builder.
      pub fn new() -> Self {
          Style::default()
      }

      /// Set a `property: value` declaration.
      pub fn set(mut self, property: impl Into<String>, value: impl Into<String>) -> Self {
          self.declarations.push((property.into(), value.into()));
          self
      }

      /// Set a declaration only when `value` is `Some`.
      pub fn set_opt(
          mut self,
          property: impl Into<String>,
          value: Option<impl Into<String>>,
      ) -> Self {
          if let Some(value) = value {
              self.declarations.push((property.into(), value.into()));
          }
          self
      }

      /// Serializes to a `style=""` attribute value. Values are emitted
      /// raw — the rendering layer escapes on emit.
      pub fn to_css_string(&self) -> String {
          self.declarations
              .iter()
              .map(|(prop, value)| format!("{prop}: {value}"))
              .collect::<Vec<_>>()
              .join("; ")
      }
  }

  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn escape_text_handles_the_three_content_characters() {
          assert_eq!(escape_text("a & b < c > d"), "a &amp; b &lt; c &gt; d");
          assert!(matches!(escape_text("plain"), Cow::Borrowed("plain")));
      }

      #[test]
      fn escape_attribute_handles_quotes_but_not_gt() {
          assert_eq!(escape_attribute(r#"he said "hi" > 'bye'"#),
              "he said &quot;hi&quot; > &#39;bye&#39;");
      }

      #[test]
      fn classes_filters_none_and_blank() {
          let out = classes([Some("a"), None, Some("  "), Some(" b ")]);
          assert_eq!(out, "a b");
      }

      #[test]
      fn css_slug_follows_the_rules() {
          assert_eq!(css_slug("Order ID"), "order-id");
          assert_eq!(css_slug("2024 totals"), "_2024-totals");
          assert_eq!(css_slug("---"), "_");
          assert_eq!(css_slug(css_slug("Order ID").as_str()), "order-id");
      }

      #[test]
      fn css_var_forms() {
          assert_eq!(css_var("brand-fg"), "var(--brand-fg)");
          assert_eq!(
              css_var_with_fallback("brand-border", "#ccc"),
              "var(--brand-border, #ccc)"
          );
      }

      #[test]
      fn style_builder_preserves_order_and_skips_none() {
          let style = Style::new()
              .set("color", "red")
              .set_opt("background", None::<String>)
              .set("padding", "1rem");
          assert_eq!(style.to_css_string(), "color: red; padding: 1rem");
      }
  }
  ```

### Task E2 — Implement `ComponentStylesheet::as_stylesheet`

**Files:**
- Modify: `renderable/src/browser/mod.rs`

- [ ] In `renderable/src/browser/mod.rs`, replace the `todo!()` body of `ComponentStylesheet::as_stylesheet` with the descendant-selector lowering described in `rendering-to-a-browser.md` (scoping rule) and the method doc. Each internal entry `(child, block)` becomes `.<name> .<child> { block }`:

  ```rust
      pub fn as_stylesheet(&self) -> Stylesheet {
          let mut out = Stylesheet::new();
          for (child_selector, block) in self.style.entries() {
              let scoped = format!(".{} .{}", self.name, child_selector);
              out.push(scoped, block.clone());
          }
          out
      }
  ```

- [ ] This requires `Stylesheet` entries (`CssStyle` blocks) to be `Clone`. Confirm the declaration-block type (`CssStyle`, or whatever Project 1 named it) derives `Clone`. If it does not, add `#[derive(Clone)]` to it — declaration blocks are plain data and cloning them is correct. Substitute `HtmlStyleSheet` for `Stylesheet` if Project 1 left that name.

### Task E3 — Implement `MetaTag::render` and fix its broken signature

**Files:**
- Modify: `renderable/src/html/tag/meta.rs`

- [ ] `renderable/src/html/tag/meta.rs` has two bugs: `set_charset` returns `&mut self` (invalid — must be `&mut MetaTag`), and `render` is `fn render() -> String { !todo() }` (`!todo()` is not valid; should be `todo!()` and `render` needs `&self`). Replace the `impl MetaTag` block with a corrected version that emits a `<meta>` tag:

  ```rust
  impl MetaTag {
      pub fn new() -> MetaTag {
          MetaTag::default()
      }

      /// Mark this meta tag as the document charset declaration.
      pub fn set_charset(&mut self) -> &mut MetaTag {
          self.charset = Some(UTF8);
          self
      }

      /// Set the `name` attribute and its `content` value.
      pub fn set_named(&mut self, name: impl Into<String>, content: impl Into<String>) -> &mut MetaTag {
          self.name = Some(name.into());
          self.content = Some(content.into());
          self
      }

      /// Renders this meta tag as an HTML `<meta>` element.
      ///
      /// Emits `<meta charset="utf-8">` when a charset is set; otherwise
      /// emits the `name` / `http-equiv` / `content` / `media` attributes
      /// that are present.
      pub fn render(&self) -> String {
          if let Some(charset) = self.charset {
              return format!(r#"<meta charset="{charset}">"#);
          }
          let mut attrs = String::new();
          if let Some(http_equiv) = &self.http_equiv {
              attrs.push_str(&format!(r#" http-equiv="{http_equiv}""#));
          }
          if let Some(name) = &self.name {
              attrs.push_str(&format!(r#" name="{name}""#));
          }
          if let Some(content) = &self.content {
              attrs.push_str(&format!(r#" content="{content}""#));
          }
          if let Some(media) = &self.media {
              attrs.push_str(&format!(r#" media="{media}""#));
          }
          format!("<meta{attrs}>")
      }
  }
  ```

### Task E4 — Implement `BrowserFragment::render` and `validate_render_content`

**Files:**
- Modify: `renderable/src/browser/fragment.rs`

- [ ] In `renderable/src/browser/fragment.rs`, replace the `todo!()` bodies in `impl BrowserFragment<Ready>` with working implementations. `render` walks the `ComposableNode` tree; `validate_render_content` checks the top-level node is present and descendants are valid. Add a private free function `render_node` that recurses. The renderer escapes `TextFragment` (via `escape_text`) and never escapes `RawHtml`:

  ```rust
  impl BrowserFragment<Ready> {
      /// Render the fragment as an HTML string.
      ///
      /// `TextFragment` content is HTML-escaped; `RawHtml` content is
      /// emitted verbatim (caller-owned). Nested `Component` fragments
      /// recurse.
      pub fn render(&self) -> String {
          match &self.node {
              Some(node) => render_node(node),
              None => String::new(),
          }
      }

      /// Returns `true` when `render()` would produce well-formed HTML:
      /// the top-level node is present, and every descendant fragment is
      /// itself valid.
      pub fn validate_render_content(&self) -> bool {
          match &self.node {
              None => false,
              Some(node) => validate_node(node),
          }
      }
  }

  /// Recursively renders a single composable node to HTML.
  fn render_node(node: &ComposableNode) -> String {
      match node {
          ComposableNode::TextFragment(text) => {
              crate::browser::utils::escape_text(text).into_owned()
          }
          ComposableNode::RawHtml(html) => html.clone(),
          ComposableNode::Component(fragment) => fragment.render(),
          ComposableNode::VoidTag(void) => {
              format!("<{}{}>", void_tag_name(&void.tag), render_attributes(&void.attributes))
          }
          ComposableNode::BlockTag(block) => {
              let name = block_tag_name(&block.tag);
              let children: String =
                  block.content.children.iter().map(render_node).collect();
              format!(
                  "<{name}{}>{children}</{name}>",
                  render_attributes(&block.attributes)
              )
          }
      }
  }

  /// Recursively validates a composable node.
  fn validate_node(node: &ComposableNode) -> bool {
      match node {
          ComposableNode::TextFragment(_) | ComposableNode::RawHtml(_) => true,
          ComposableNode::VoidTag(_) => true,
          ComposableNode::Component(fragment) => fragment.validate_render_content(),
          ComposableNode::BlockTag(block) => {
              block.content.children.iter().all(validate_node)
          }
      }
  }
  ```

- [ ] The functions above reference `render_attributes`, `void_tag_name`, and `block_tag_name`. Add `render_attributes` as a private free function in the same file. It serializes an attribute slice, escaping values via `escape_attribute`. Keep it deliberately small — it covers the common attribute variants and falls back to a best-effort string for the rest:

  ```rust
  /// Serializes a slice of attributes into an opening-tag attribute string
  /// (leading space included when non-empty). Attribute values are escaped.
  fn render_attributes(attributes: &[HtmlAttribute]) -> String {
      let mut out = String::new();
      for attr in attributes {
          let pair: Option<(&str, String)> = match attr {
              HtmlAttribute::Title(value) => Some(("title", value.clone())),
              HtmlAttribute::Alt(value) => Some(("alt", value.clone())),
              HtmlAttribute::Name(value) => Some(("name", value.clone())),
              HtmlAttribute::Placeholder(value) => Some(("placeholder", value.clone())),
              HtmlAttribute::Target(value) => Some(("target", value.clone())),
              HtmlAttribute::Href(url) => Some(("href", url.to_string())),
              HtmlAttribute::Src(url) => Some(("src", url.to_string())),
              _ => None,
          };
          if let Some((key, value)) = pair {
              out.push_str(&format!(
                  r#" {key}="{}""#,
                  crate::browser::utils::escape_attribute(&value)
              ));
          }
      }
      out
  }
  ```

- [ ] `void_tag_name` and `block_tag_name` map the `VoidTag` / `BlockTag` enums to their lowercase HTML tag names. The `BlockTag` enum has ~110 variants and `VoidTag` has 14. Adding two full match functions is the correct approach but is large. Instead, add them to `renderable/src/html/tag/mod.rs` as methods on the enums so the tag-name knowledge lives next to the enum definition. In `renderable/src/html/tag/mod.rs`, add an `impl VoidTag` and `impl BlockTag` block — each `name(&self) -> &'static str` matching every variant to its tag string (e.g. `VoidTag::Br => "br"`, `BlockTag::Div => "div"`, `BlockTag::H1 => "h1"`, `BlockTag::FigCaption => "figcaption"`, `BlockTag::OptGroup => "optgroup"`). Then in `fragment.rs`, replace the `void_tag_name(&void.tag)` / `block_tag_name(&block.tag)` calls with `void.tag.name()` / `block.tag.name()` and drop the two free-function stubs.

  > **Implementer note:** the tag-name match is mechanical — one arm per enum variant, lowercased, with the camel-case variants joined (`FigCaption → "figcaption"`, `DatetimeLocal` is on `HtmlType` not a tag so ignore it). Do not abbreviate; emit every arm explicitly so the compiler's exhaustiveness check catches a missed variant.

### Task E5 — Implement `HtmlPage::render`, `stylesheet`, and `inline_code`

**Files:**
- Modify: `renderable/src/html/mod.rs`

- [ ] Replace the three `todo!()` bodies in `impl HtmlPage`. `stylesheet()` rolls up the `:root` block, the page stylesheet, and component default stylesheets. `inline_code()` rolls up script blocks. `render()` assembles the document with the fixed `<head>` ordering from `rendering-to-a-browser.md` § "`<head>` Ordering". Add the implementations:

  ```rust
      /// Returns the page's rolled-up CSS text.
      ///
      /// Order: `:root` semantic-token block, then the page stylesheet,
      /// then each fragment's component default stylesheet in
      /// fragment-registration order.
      pub fn stylesheet(&self) -> String {
          let mut out = String::new();
          out.push_str(&self.render_root_block());
          out.push_str(&render_stylesheet(&self.stylesheet));
          for fragment in &self.fragments {
              if let Some(component_sheet) = fragment.stylesheet() {
                  out.push_str(&render_stylesheet(&component_sheet.as_stylesheet()));
              }
          }
          out
      }

      /// Returns the page's rolled-up JS text (page-level script blocks,
      /// joined by blank lines).
      pub fn inline_code(&self) -> String {
          self.script_blocks.join("\n\n")
      }

      /// Renders the page to a complete HTML string. Pure — no I/O.
      pub fn render(&self) -> String {
          let mut head = String::new();
          // 1. charset — always first.
          head.push_str(r#"<meta charset="utf-8">"#);
          // 2. viewport.
          head.push_str(
              r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#,
          );
          // 3. title — from the Title microdata key, else first <h1>, else empty.
          let title = self
              .metadata
              .get(&MicrodataKey::Title)
              .cloned()
              .or_else(|| self.first_h1_text())
              .unwrap_or_default();
          head.push_str(&format!(
              "<title>{}</title>",
              crate::browser::utils::escape_text(&title)
          ));
          // 4. other microdata-driven meta tags.
          for (key, value) in &self.metadata {
              if *key == MicrodataKey::Title {
                  continue;
              }
              for (_source, html) in crate::microdata::microdata(*key, value) {
                  for tag in html {
                      head.push_str(&tag);
                  }
              }
          }
          // 5. link tags — deduped, page-level first.
          for link in self.collect_dedup_links() {
              head.push_str(&link.render());
          }
          // 6. stylesheet — external <link> or inline <style>.
          match &self.external_stylesheet {
              Some(path) => head.push_str(&format!(
                  r#"<link rel="stylesheet" href="{}">"#,
                  path.display()
              )),
              None => {
                  let css = self.stylesheet();
                  if !css.is_empty() {
                      head.push_str(&format!("<style>{css}</style>"));
                  }
              }
          }
          // 7. script blocks — external <script src> or inline <script>.
          match &self.external_code {
              Some(path) => head.push_str(&format!(
                  r#"<script src="{}" defer></script>"#,
                  path.display()
              )),
              None => {
                  let code = self.inline_code();
                  if !code.is_empty() {
                      head.push_str(&format!("<script>{code}</script>"));
                  }
              }
          }
          let body: String = self.fragments.iter().map(|f| f.render()).collect();
          format!(
              "<!DOCTYPE html><html><head>{head}</head><body>{body}</body></html>"
          )
      }
  ```

- [ ] The `render()` body above references three helpers: `HtmlPage::render_root_block`, `HtmlPage::first_h1_text`, and the free function `render_stylesheet`, plus `LinkTag::render`. Add `render_root_block` and `first_h1_text` as private methods on `HtmlPage`:

  ```rust
      /// Renders the page-level `:root { … }` block. Starts from the
      /// semantic-token defaults and applies any `css_variables` overrides.
      fn render_root_block(&self) -> String {
          let mut variables: Vec<(String, String)> = crate::tokens::root_defaults();
          if let Some(overrides) = &self.css_variables {
              for (name, value) in overrides {
                  match variables.iter_mut().find(|(n, _)| n == name) {
                      Some(entry) => entry.1 = value.clone(),
                      None => variables.push((name.clone(), value.clone())),
                  }
              }
          }
          let body: String = variables
              .iter()
              .map(|(name, value)| format!("--{name}: {value};"))
              .collect();
          format!(":root{{{body}}}")
      }

      /// Walks the fragment tree for the first `<h1>` and returns its
      /// concatenated text content. `RawHtml` islands are invisible to
      /// this scan (decisions.md item 8B).
      fn first_h1_text(&self) -> Option<String> {
          for fragment in &self.fragments {
              if let Some(node) = fragment.node() {
                  if let Some(text) = crate::html::find_first_h1_text(node) {
                      return Some(text);
                  }
              }
          }
          None
      }
  ```

- [ ] Add the free function `render_stylesheet` and `find_first_h1_text` at module level in `renderable/src/html/mod.rs`. `render_stylesheet` turns a `Stylesheet` collection into CSS text; `find_first_h1_text` recurses the node tree:

  ```rust
  use crate::browser::fragment::ComposableNode;
  use crate::html::tag::BlockTag;

  /// Serializes a `Stylesheet` collection into CSS rule text.
  fn render_stylesheet(sheet: &Stylesheet) -> String {
      sheet
          .entries()
          .iter()
          .map(|(selector, block)| format!("{selector}{{{}}}", block.to_css()))
          .collect()
  }

  /// Recursively searches a composable node tree for the first `<h1>` and
  /// returns its concatenated text content. Returns `None` if no `<h1>` is
  /// found in the typed tree (`RawHtml` islands are not scanned).
  pub(crate) fn find_first_h1_text(node: &ComposableNode) -> Option<String> {
      match node {
          ComposableNode::BlockTag(block) => {
              if matches!(block.tag, BlockTag::H1) {
                  return Some(collect_text(&block.content.children));
              }
              for child in &block.content.children {
                  if let Some(text) = find_first_h1_text(child) {
                      return Some(text);
                  }
              }
              None
          }
          ComposableNode::Component(fragment) => {
              fragment.node().and_then(find_first_h1_text)
          }
          _ => None,
      }
  }

  /// Concatenates the text content of a node slice (text fragments only).
  fn collect_text(children: &[ComposableNode]) -> String {
      let mut out = String::new();
      for child in children {
          match child {
              ComposableNode::TextFragment(text) => out.push_str(text),
              ComposableNode::BlockTag(block) => {
                  out.push_str(&collect_text(&block.content.children));
              }
              ComposableNode::Component(fragment) => {
                  if let Some(node) = fragment.node() {
                      if let ComposableNode::BlockTag(block) = node {
                          out.push_str(&collect_text(&block.content.children));
                      } else if let ComposableNode::TextFragment(text) = node {
                          out.push_str(text);
                      }
                  }
              }
              _ => {}
          }
      }
      out
  }
  ```

- [ ] `render_stylesheet` calls `block.to_css()` on the declaration-block type. Project 1's stylesheet extraction provides `CssStyle::to_css` (the `to_css` emit method moved from `darkmatter`). Confirm the method name with `grep -rn "fn to_css" renderable/src`; if Project 1 named it differently, substitute. `entries()` returns `&[(String, CssStyle)]` per the collection type.

### Task E5b — Add `LinkTag::render`

**Files:**
- Modify: `renderable/src/html/tag/link.rs`

- [ ] `HtmlPage::render` calls `LinkTag::render`, which does not exist. Add it to `impl LinkTag` in `renderable/src/html/tag/link.rs`. It emits a `<link>` tag with the `rel`, `href`, and optional attributes. Reuse the `rel`-string match already inside `dedup_key` by extracting it — add a private `rel_str` method and have both `dedup_key` and `render` call it:

  ```rust
      /// Returns the `rel` attribute as its HTML keyword string.
      fn rel_str(&self) -> &'static str {
          match self.rel {
              LinkRel::Alternate => "alternate",
              LinkRel::Author => "author",
              LinkRel::Canonical => "canonical",
              LinkRel::CompressionDictionary => "compression-dictionary",
              LinkRel::Expect => "expect",
              LinkRel::Help => "help",
              LinkRel::License => "license",
              LinkRel::Manifest => "manifest",
              LinkRel::Me => "me",
              LinkRel::Next => "next",
              LinkRel::Prev => "prev",
              LinkRel::PrivacyPolicy => "privacy-policy",
              LinkRel::Search => "search",
              LinkRel::Stylesheet => "stylesheet",
              LinkRel::TermsOfService => "terms-of-service",
          }
      }

      /// Renders this link tag as an HTML `<link>` element.
      pub fn render(&self) -> String {
          let mut out = format!(r#"<link rel="{}""#, self.rel_str());
          if let Some(href) = &self.href {
              out.push_str(&format!(r#" href="{href}""#));
          }
          if let Some(hreflang) = &self.hreflang {
              out.push_str(&format!(r#" hreflang="{hreflang}""#));
          }
          if let Some(title) = &self.title {
              out.push_str(&format!(r#" title="{title}""#));
          }
          if let Some(media) = &self.media {
              out.push_str(&format!(r#" media="{media}""#));
          }
          out.push('>');
          out
      }
  ```

- [ ] Update `dedup_key` to call `self.rel_str()` instead of repeating the match: change its body to `format!("{}|{}", self.rel_str(), self.href.as_deref().unwrap_or(""))`.

### Task E6 — Integration tests for the render pipeline

**Files:**
- Create: `renderable/tests/render_pipeline.rs`

- [ ] Add an integration test exercising the full pipeline: build fragments, compose a page, render, and assert structural properties. Create `renderable/tests/render_pipeline.rs`:

  ```rust
  use renderable::browser::PageOptions;
  use renderable::browser::fragment::BrowserFragment;
  use renderable::html::HtmlPage;
  use renderable::html::tag::BlockTag;
  use renderable::microdata::MicrodataKey;

  #[test]
  fn fragment_renders_block_with_text_child() {
      let fragment = BrowserFragment::new()
          .define_as_block_tag(BlockTag::P, "intro")
          .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
              "hello & welcome".to_string(),
          ))
          .finalize();
      let html = fragment.render();
      assert_eq!(html, "<p>hello &amp; welcome</p>");
  }

  #[test]
  fn raw_html_is_not_escaped() {
      let fragment = BrowserFragment::new()
          .define_as_raw_html("<svg><rect/></svg>")
          .finalize();
      assert_eq!(fragment.render(), "<svg><rect/></svg>");
  }

  #[test]
  fn page_render_emits_doctype_charset_and_title() {
      let body = BrowserFragment::new()
          .define_as_block_tag(BlockTag::H1, "heading")
          .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
              "Welcome".to_string(),
          ))
          .finalize();
      let mut page = HtmlPage::from(body);
      page.set_title("My Page");
      let html = page.render();
      assert!(html.starts_with("<!DOCTYPE html><html><head><meta charset=\"utf-8\">"));
      assert!(html.contains("<title>My Page</title>"));
      assert!(html.contains("<h1>Welcome</h1>"));
  }

  #[test]
  fn title_falls_back_to_first_h1() {
      let body = BrowserFragment::new()
          .define_as_block_tag(BlockTag::H1, "heading")
          .add_child(renderable::browser::fragment::ComposableNode::TextFragment(
              "Derived Title".to_string(),
          ))
          .finalize();
      let page = HtmlPage::from(body);
      assert!(page.render().contains("<title>Derived Title</title>"));
  }

  #[test]
  fn page_emits_root_block_with_semantic_tokens() {
      let body = BrowserFragment::new()
          .define_as_text_fragment("content")
          .finalize();
      let page = HtmlPage::from(body);
      let css = page.stylesheet();
      assert!(css.contains("--color-bg:"));
      assert!(css.contains("--space-2:"));
      assert!(css.contains("--font-mono:"));
  }

  #[test]
  fn page_options_override_a_css_variable() {
      let body = BrowserFragment::new()
          .define_as_text_fragment("content")
          .finalize();
      let mut page = HtmlPage::from(body);
      page.apply_page_options(PageOptions {
          css_variables: Some(vec![("color-bg".to_string(), "#000000".to_string())]),
          ..PageOptions::default()
      });
      assert!(page.stylesheet().contains("--color-bg: #000000;"));
  }

  #[test]
  fn external_stylesheet_emits_a_link_not_inline_style() {
      let body = BrowserFragment::new()
          .define_as_text_fragment("content")
          .finalize();
      let mut page = HtmlPage::from(body);
      page.apply_page_options(PageOptions {
          external_stylesheet: Some(std::path::PathBuf::from("assets/page.css")),
          ..PageOptions::default()
      });
      let html = page.render();
      assert!(html.contains(r#"<link rel="stylesheet" href="assets/page.css">"#));
      assert!(!html.contains("<style>"));
  }
  ```

- [ ] If any module path in the test (`renderable::browser::fragment::ComposableNode`, `renderable::html::tag::BlockTag`, etc.) does not resolve, adjust the `use` paths to match the actual public module layout — confirm with `cargo doc -p renderable --no-deps` or by reading `lib.rs` / `browser/mod.rs`. The test logic itself is correct; only the paths may need tuning.

### Task E7 — Verify and commit Phase E

**Files:** none (verification only)

- [ ] Run `cargo build -p renderable`. Expected: `Finished` with no errors. The whole crate now compiles with every `todo!()` replaced.
- [ ] Run `cargo nextest run -p renderable` (fallback `cargo test -p renderable`). Expected: all unit tests and the `render_pipeline` integration tests pass, `0 failed`.
- [ ] Run `cargo test -p renderable --doc`. Expected: all doc-tests pass — the new trait doc comments contain intra-doc links that must resolve.
- [ ] Run `cargo doc -p renderable --no-deps`. Expected: no new warnings; intra-doc links in `BrowserRenderable`, `MarkdownRenderable`, `tokens`, and `HtmlPage` resolve.
- [ ] Stage and commit:

  ```bash
  git add renderable/src/browser/utils.rs renderable/src/browser/mod.rs \
    renderable/src/browser/fragment.rs renderable/src/html/mod.rs \
    renderable/src/html/tag/mod.rs renderable/src/html/tag/meta.rs \
    renderable/src/html/tag/link.rs renderable/tests/render_pipeline.rs
  git commit -m "feat(renderable): implement fragment and page render pipeline"
  ```

  Expected: commit succeeds. No `Co-Authored-By` trailer.

---

## Verification

Project 2 is complete when all of the following hold:

- [ ] `cargo build -p renderable` succeeds with no errors or warnings.
- [ ] `cargo nextest run -p renderable` (fallback `cargo test -p renderable`) — all tests pass, `0 failed`, including the `tokens`, `browser::utils`, `browser::fragment`, and `render_pipeline` test suites.
- [ ] `cargo test -p renderable --doc` — all doc-tests pass.
- [ ] `cargo doc -p renderable --no-deps` — no new warnings; all intra-doc links resolve.
- [ ] `renderable` is still a leaf crate: `cargo tree -p renderable -i biscuit-terminal` and `-i darkmatter` find no match.
- [ ] `BrowserRenderable` carries four methods (`render_to_browser`, `render_to_browser_with_inline_variables`, `render_html_fragment`, `render_html_page`); the latter two have default implementations.
- [ ] `MarkdownRenderable` and `AstRenderable` traits exist and are exported.
- [ ] `ComposableNode` has five variants including `RawHtml(String)` and `Component(Box<BrowserFragment<Ready>>)` (boxed to break the otherwise-infinite recursive type — `BrowserFragment` itself stores a `ComposableNode`); `BrowserFragment::define_as_raw_html` and `add_component` exist.
- [ ] The semantic token layer (`SemanticToken`, `SpaceToken`, `FontToken`, `root_defaults`) exists in `renderable::tokens` and is compiler-checked.
- [ ] `PageOptions` carries no `Layout`; it has `stylesheet`, `css_variables`, `external_stylesheet`, `external_code`.
- [ ] `HtmlPage` has no `title` field; `set_title` writes the `Title` microdata key; `render`, `stylesheet`, and `inline_code` are pure and perform no I/O.
- [ ] `BrowserFragment::render` escapes `TextFragment` and never escapes `RawHtml`.

## Risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Project 1 left the collection type named `HtmlStyleSheet` (Scheme A rename is Project 3) | High | Every code block flags the substitution; confirm the real name with `grep` before each affected step. |
| `renderable/src/browser/renderable.rs` does not compile until Phase D | Certain (by design) | Phases A–C defer the build; Phase D Task D4 is the first green checkpoint. Do not attempt `cargo build` before D4. |
| `BlockTag` / `VoidTag` `name()` match misses a variant | Medium | Exhaustive match — the compiler's non-exhaustive-pattern error catches it. Emit every arm explicitly, no wildcard. |
| `CssStyle` declaration block lacks `Clone` (needed by `as_stylesheet`) | Medium | Task E2 adds `#[derive(Clone)]` if absent — declaration blocks are plain data. |
| `to_css` method on the declaration block named differently by Project 1 | Low | Task E5 flags a `grep` confirmation step. |
| Microdata `Title` fan-out double-emits `<title>` (microdata module emits one, `render()` emits another) | Medium | `render()` skips `MicrodataKey::Title` in the step-4 loop and emits `<title>` itself in step 3. |

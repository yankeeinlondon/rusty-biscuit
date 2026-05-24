# BrowserRenderable Refactor — Decisions

Authoritative record of the decisions taken while working through
[`brainstorming.md`](./brainstorming.md). Each of the 12 brainstorming items
is resolved below. Where a decision changes an earlier doc, the affected doc
has been reconciled to match — see the **Doc reconciliation** notes.

This document supersedes conflicting statements in `brainstorming.md`,
`spec.md`, and `rendering-to-a-browser.md`.

---

## 1. Composition & merge semantics

**Decision.** Composition is structural, not a merge operation. A parent
component nests a child via `ComposableNode::Component`, and that variant
holds a fully-rendered `BrowserFragment<Ready>` (eager, not a boxed
`dyn BrowserRenderable`):

```rust
pub enum ComposableNode {
    BlockTag(HtmlBlockTag),
    VoidTag(HtmlVoidTag),
    TextFragment(String),
    RawHtml(String),                 // added — see item 2
    Component(BrowserFragment<Ready>),
}
```

There is no `absorb` / `compose` / `FragmentAux` helper. The whole node tree
is homogeneous — `BrowserFragment<Ready>` is the universal "done" currency.

**Aggregation** (rolling up stylesheets, features, metadata, dependency
links) is a page-level concern: `HtmlPage` walks its fragment tree, descending
through `Component` nodes, and collects auxiliary state. Because every nested
component is already a `BrowserFragment<Ready>`, this is a pure recursive walk
with no trait calls and no double-render.

**Rationale.** Eager `BrowserFragment<Ready>` makes the tree uniform: no
caching, no two-pass ordering rules, no silent data-loss foot-gun.

## 2. Migration of `render_to_browser`

**Decision (2A).** The `BrowserRenderable` trait carries four methods during
Project 2 (the coexistence window), exactly as `spec.md` Project 2 intends —
the implemented single-method trait is reconciled back to this shape:

```rust
pub trait BrowserRenderable: std::fmt::Debug + Any {
    // pre-existing — deprecated, removed in Project 3
    fn render_to_browser(&self) -> String;
    fn render_to_browser_with_inline_variables(
        &self, variables: &HashMap<String, String>,
    ) -> String;
    // new surface
    fn render_html_fragment(&self) -> BrowserFragment<Ready>;
    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage;
}
```

Note `render_html_page` returns `HtmlPage`, **not** `String` (see item 7).

**Decision (2B).** A new `ComposableNode::RawHtml(String)` variant is the
escape hatch for content that genuinely *is* a prebuilt string (SVG,
third-party HTML). A `define_as_raw_html(s)` builder produces it. `RawHtml`
content is **caller-owned and never escaped** by the renderer — distinct from
`TextFragment`, which is escaped on emit.

`render_to_browser_with_inline_variables` is retained through Project 2 and
deleted in Project 3; no per-component variable hook survives — components
emit `var(--foo)` literally and the page declares the variable.

## 3. CSS variables — palette vs semantic tokens

**Decision (3A).** `renderable` ships **both** a palette layer
(`--color-blue-500: #3b82f6`, mechanically derived from Tailwind) **and** a
curated semantic layer (`--color-bg`, `--color-fg`, `--color-error`, …) whose
defaults reference palette tokens. Components reference the semantic layer;
callers re-theme by overriding semantic tokens via `PageOptions`.

**Decision (3B).** Three token families ship now: **colors, spacing,
typography**. Radius, z-index, and animation durations are deferred until a
component needs them.

**Decision (3C).** Tailwind-style namespaced naming: `--color-bg`, `--space-2`,
`--font-mono`. Consistent prefix per family.

**Decision (3D).** The semantic layer is compiler-checked. Typed enums —
`SemanticToken`, `SpaceToken`, `FontToken` — are the single source of truth
and double as the generator for the `:root` defaults. `Color::Var(String)`
survives for palette tokens and arbitrary caller-defined variables, where an
open set is correct.

## 4. Body content & sanitization

Largely settled by `browser-utils.md`: `escape_text` / `escape_attribute`
exist, escaping happens **on emit** in the render layer.

| Node | Escaping |
|---|---|
| `TextFragment(String)` | escaped on emit |
| `RawHtml(String)` | never — caller owns it |
| attribute values | escaped on emit |

**Decision (4A).** `Prose` lowers its tokens into a typed `ComposableNode`
tree (`<strong>`, `<em>`, `<a>`) — escaping is then automatic. `Markdown`
lowers into typed nodes too, but keeps `RawHtml` islands for embedded inline
HTML (the "MarkdownPlus" case).

**Decision (4B).** The spec gains one non-binding line: components SHOULD use
semantic tags and ARIA attributes where appropriate.

## 5. CSS variable scope

**Decision (5A).** Component-scoped custom properties are explicitly allowed —
a component may declare `.wrapper { --row-pad: 4px; }` and consume
`var(--row-pad)` in descendant rules. This is class-scoped, never `:root`, so
it does not collide with the page-level model. A `CssStyle` declaration block
permits custom-property declarations like any other.

**Decision (5B).** Page-level `:root` is declared by the page only; components
consume. A component emitting a `:root` block (only possible by smuggling it
through `RawHtml`) is an anti-pattern handled by **documentation only** — no
runtime scan.

## 6. `Layout` in HTML context

**Decision.** `PageOptions` does **not** carry a `Layout`. Page-level
background, margins, and padding are expressed entirely through the page
`Stylesheet` (rulesets on `html` / `body`). `layout` and `stylesheet` were two
overlapping mechanisms for one job; `stylesheet` is strictly more capable for
HTML, so `Layout` is removed from `PageOptions`.

`Layout` still moves from `biscuit-terminal` to `renderable` per
`layout-and-color-move.md` — it remains a future cross-target layout
primitive — it is simply not wired into `PageOptions`. Items 6A/6B (where a
page background color is applied, terminal-only `Color` variants) are moot.

## 7. External dependency strategy

**Decision (7A).** Real `<link>` dependencies (`BrowserFragment`
`dependency_links`) are always emitted as `<link>` — they are external by
nature. The inline-vs-external choice applies only to the page's *own*
rolled-up CSS and JS, which `renderable` holds as text.

**Decision.** `render_html_page` returns an `HtmlPage` struct (not a string).
The page model and final rendering are separated:

```rust
impl HtmlPage {
    fn render(&self) -> String;       // pure: HTML string only, no I/O
    fn stylesheet(&self) -> String;   // rolled-up CSS text
    fn inline_code(&self) -> String;  // rolled-up JS text
}
```

`render()` is a pure string transform and never writes files. When
`PageOptions` selects external assets, `render()` emits `<link>` / `<script
src>` references; the caller pulls content from `stylesheet()` /
`inline_code()` and writes it.

**Decision.** The inline-vs-external choice lives on `PageOptions` (a single
options struct — no separate `RenderOptions`):

```rust
pub struct PageOptions {
    stylesheet: Option<Stylesheet>,        // collection type — see item 10
    css_variables: Option<Vec<(String, String)>>,
    external_stylesheet: Option<PathBuf>,  // None → inline <style>
    external_code: Option<PathBuf>,        // None → inline <script>
}
```

`external_stylesheet` / `external_code` paths are enforced **relative** so the
`href` / `src` stays portable.

**Decision (7F).** `HtmlPage::render()` is argument-less — everything it needs
was decided at page-assembly time. A render-time-only option (e.g.
minify vs pretty-print) is speculative and deferred (YAGNI).

**Decision (7B).** Version conflicts between dependency links (same library,
different version → different `href`, not caught by the `(rel, href)` dedup
key) are **accepted and documented** as a known limitation. URL-based version
heuristics are too fragile to be trustworthy.

## 8. Page title & metadata

**Decision (8A).** `HtmlPage` does not store a `title` field. A `set_title()`
method writes the `Title` microdata key, so the title gets the full HTML /
OpenGraph / Twitter / Schema.org fan-out automatically. One code path.

**Metadata model.** `HtmlPage` **owns** all metadata. Components accumulate
metadata that bubbles up the fragment tree; callers also set metadata at the
page level (the most common place). At `render()` time `HtmlPage` merges all
metadata:

- **Page-level metadata always wins** on key conflict.
- **Component-vs-component conflict: first-write wins** (document order — the
  earlier component is the more primary one). *(8D)*

After merging, if `title` is unset, the renderer derives it from the first
`<h1>` on the page.

**Decision (8B).** The first-`<h1>` scan walks the typed `ComposableNode`
tree, concatenating the heading's text fragments. An `<h1>` inside a `RawHtml`
island is invisible to the scan — a `RawHtml`-bodied component sets `title`
explicitly if it cares. Documented.

**Decision (8C).** When neither microdata `Title` nor any `<h1>` resolves, the
renderer emits an empty `<title></title>` so every page stays spec-valid.
There is no SEO advantage to omitting it.

## 9. Inline-style vs class overrides

**Decision.** The spec wording is corrected — instance `style` takes
precedence over component class defaults *in typical use*, but CSS specificity
ultimately decides. The library **never emits `!important`**; a caller that
needs it writes it into its own `style` / stylesheet value explicitly.

## 10. Naming — stylesheet type hierarchy

**Decision (Scheme A).** CSS-accurate naming across the three concepts:

| Concept | Old name | New name |
|---|---|---|
| declaration block (`{ prop: val; … }`, also inline `style=""`) | `Stylesheet` | **`CssStyle`** |
| a single `selector { block }` rule | *(the `(String, Stylesheet)` pair)* | **`CssRule`** |
| collection of rules | `HtmlStyleSheet` | **`Stylesheet`** |

`HtmlClassDefinition` / `ClassDefinition` (a `class=""` attribute value) is
**left alone** — it is a list of class names, not a stylesheet. `spec.md`
Project 3's proposed `HtmlClassDefinition → Stylesheet` rename is **dropped**;
it was the source of the naming confusion.

## 11. Anti-pattern & convention list

**Decision (11A).** Enforcement is **documentation only**. The typed node tree
already prevents the easy mistakes structurally; string-scanning `RawHtml`
islands is brittle and low-value. `validate_render_content()` stays focused on
well-formedness, not convention policing.

**Decision (11B).** The convention list:

1. Never emit `!important` from builder / renderer APIs.
2. Reference the semantic token layer (`SemanticToken` / `SpaceToken` /
   `FontToken`), not palette tokens, in component CSS.
3. Do not declare page-level `:root` variables from a component;
   component-scoped custom properties on the wrapper class are fine.
4. `RawHtml` content is caller-owned and unescaped — only pass final,
   escaped markup.
5. Do not depend on `<head>` ordering beyond the Render Pipeline guarantees.
6. Do not smuggle `<style>` / `<script>` / `:root` through `RawHtml` — use the
   `stylesheet` / features / page-level mechanisms.

## 12. Migration of existing implementors

**Decision (12A).** `DarkmatterPage` is **not** a `BrowserRenderable`. It is a
page assembler — it consumes many fragments and produces an `HtmlPage`. A page
is a different role from a component (a component produces one
`BrowserFragment`). `DarkmatterPage` becomes an `HtmlPage` builder with no
trait.

> Longer-term goal: replace `DarkmatterPage` with a unified `Darkmatter`
> *component* that implements both `TerminalRenderable` and
> `BrowserRenderable`.

This does **not** remove `render_html_page` from the trait —
`render_html_page` means "promote *this one component* to a standalone
single-fragment page" (`HtmlPage::from(self.render_html_fragment())` plus
options), which is unrelated to `DarkmatterPage`'s multi-fragment assembly.

**Decision (12B).** Component-by-component migration during the Project 2
coexistence window. The three string-producing implementors each get a
one-line `render_html_fragment`:

```rust
fn render_html_fragment(&self) -> BrowserFragment<Ready> {
    BrowserFragment::new()
        .define_as_raw_html(self.render_to_browser())
        .finalize()
}
```

| Component | Migration |
|---|---|
| `HorizontalRule` | `RawHtml`; `var(--…)` placeholders stay literal in the SVG, page declares the variables |
| `GraphExpression` | `RawHtml`; no scripts or dependencies |
| `YamlBlock` | `RawHtml`; gains a `CssStyle` later if syntax highlighting is added |
| `DarkmatterPage` | drops `BrowserRenderable`; becomes an `HtmlPage` builder |

---

## Doc reconciliation

The following docs were updated to match these decisions:

- **`brainstorming.md`** — banner added marking all 12 items resolved here.
- **`spec.md`** — Project 2 trait block: `render_html_page` returns
  `HtmlPage`; `render_html_fragment` returns `BrowserFragment<Ready>`.
  Project 3 naming housekeeping replaced with Scheme A (item 10).
- **`rendering-to-a-browser.md`** — `BrowserRenderable` trait signature and the
  page-assembly example updated; `PageOptions` shape noted.
- **`layout-and-color-move.md`** — note added that `Layout` is not consumed by
  `PageOptions`; `Stylesheet` → `CssStyle` per item 10.
- **`stylesheet-extraction.md`** — Issue 5 (naming) resolved by Scheme A.

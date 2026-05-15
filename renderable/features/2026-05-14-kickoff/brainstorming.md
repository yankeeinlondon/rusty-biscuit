# Brainstorming: BrowserRenderable Open Questions

Discussion agenda for the followup conversation on [`rendering-to-a-browser.md`](./rendering-to-a-browser.md). Items already landed (descendant scoping, dedup at page assembly, ordered collections, owned fragments, render-pipeline section, code-feature enum stub) are **not** repeated here — this doc only covers what still needs a decision or a design pass.

Sorted roughly by priority: composition/merge semantics is the largest open item and should be discussed first.

---

## 1. Composition & Merge Semantics  (top priority)

When a parent component renders, it calls `child.render_html_fragment()` and receives a `BrowserFragment`. The parent then needs to:

- Embed the child's `body` string inside its own HTML.
- Carry forward the child's `stylesheet`, `metadata`, `dependency_links`, and `code_features` so the page-level assembler eventually sees them.

Today there is no helper for this. Every parent component would reimplement the merge, and they will not all do it the same way.

### Discussion points

- **Should there be a `BrowserFragment::compose(self, child: BrowserFragment) -> Self` (or similar)?** What's the signature? Owned-into-owned? `&mut self, child: BrowserFragment`? A trait?
- **Collision rules** for each aux field when merging:
    - `body` — the parent's body is the canonical output; the child's body becomes an inline string to embed wherever the parent wants. The merge probably does *not* concatenate `body` automatically; the parent decides placement.
    - `stylesheet` (component default styles) — does the parent take ownership of all children's component stylesheets? Are they namespaced separately so they don't collide?
    - `metadata` — first-write or last-write wins on collision? (Microdata `Title` from a nested component probably shouldn't override the parent's.)
    - `dependency_links` — concat, dedup happens at page assembly. Cheap.
    - `code_features` — concat, dedup at page assembly (set semantics — `CopyToClipboard` registered twice is still once).
- **Recursive composition** — Section contains an OrderedList that contains an InlineContent. Does each level call `compose` or does the leaf-most child propagate up via some other mechanism (e.g. a builder context)?
- **Composition vs aggregation** — do we want a separate "I'm assembling several siblings" operation (page assembly does this for `Vec<BrowserFragment>`) vs "I'm nesting one inside another" (parent component does this)?

### Proposed strawman (to react to, not commit to)

```rust
impl BrowserFragment {
    /// Embed a child fragment's auxiliary state (stylesheet, metadata,
    /// dependencies, code features) into this fragment. The child's `body`
    /// string is *returned* for the parent to place wherever it wants in
    /// its own HTML output.
    pub fn absorb(&mut self, child: BrowserFragment) -> Option<String>;
}
```

Discuss: is `absorb` the right verb? Is returning the body string the right ergonomic? Should it be `(body, fragment)` so the parent can keep building, or should we make `body` a separate concern from aux state entirely?

---

## 2. What Does Today's `render_to_browser()` Become?

Existing components (`HorizontalRule`, `GraphExpression`, `YamlBlock`, `DarkmatterPage`) implement `BrowserRenderable::render_to_browser() -> String`. The new spec adds `render_html_fragment() -> BrowserFragment`.

### Discussion points

- **Coexistence vs replacement.** Does the new trait method *replace* `render_to_browser`, or coexist with it (with a default implementation bridging them)?
- **Bridge default impl.** A reasonable default would be:

    ```rust
    fn render_html_fragment(&self) -> BrowserFragment {
        BrowserFragment {
            body: Some(self.render_to_browser()),
            ..BrowserFragment::default()
        }
    }
    ```

    This lets existing implementors compile unchanged. They get an empty aux state, which is fine for components that don't have their own stylesheet/dependencies.

- **`render_to_browser_with_inline_variables`** — only `HorizontalRule` overrides this today. What's its role in the new world? Options:
    1. Deprecate; CSS variables are page-level concerns, components shouldn't override.
    2. Keep as a render-time hook that gets called by the page assembler once variables are known.
    3. Move responsibility to a new method on `BrowserFragment` itself (`fragment.with_resolved_variables(&map)`).
- **Migration sequencing.** Is the plan to add `render_html_fragment` first (with the bridge default) and migrate component-by-component, or do all components flip together?

---

## 3. CSS Variables: Semantic Tokens vs Palette Tokens

The current spec mentions injecting Tailwind colors as CSS variables. That covers the **palette** layer. What's missing is the **semantic** layer.

### Discussion points

- **Palette vs semantic.** Tailwind gives us `--color-blue-500: #3b82f6;`. But components should usually reference *semantic* tokens like `--color-bg`, `--color-fg`, `--color-error`, `--color-border-subtle`. The semantic token resolves to a palette token (`--color-error: var(--color-red-500);`). This indirection is what makes theming / dark-mode practical.
- **Do we ship semantic defaults?** Or do we only emit the palette and let downstream callers wire up semantic mappings? Defaults are convenient; absence is more flexible.
- **Beyond colors**, the spec asks "what other things are good candidates for CSS variables?" My nominations:
    - Spacing scale (`--space-1`, `--space-2`, …) — most layouts use a scale, not arbitrary px values.
    - Typography (`--font-mono`, `--font-prose`, `--text-sm`, `--text-base`, `--text-lg`).
    - Border-radius scale (`--radius-sm`, `--radius-md`, `--radius-full`).
    - Z-index layers (`--z-modal`, `--z-tooltip`, `--z-toast`).
    - Animation durations (`--duration-fast`, `--duration-slow`).
- **Naming convention.** Tailwind uses `--color-{name}-{shade}`. shadcn uses HSL components in `--background`. Pick one and stick with it.

---

## 4. Body Content & Sanitization

`BrowserFragment.body: Option<String>` is a raw HTML string. There is no escaping discipline in the type itself.

### Discussion points

- **Sanitization expectations.** Is the component responsible for HTML-escaping all user-provided text it embeds? (Almost certainly yes — but the spec should say so.)
- **Helper API.** Do we provide a safe-by-default API for emitting text? Some options:
    - A `body_text(s)` helper that escapes and pushes.
    - A typed body (e.g. `Body::Element(Tag, Vec<Body>)` AST) that escapes during serialization. Heavy but eliminates the foot-gun.
    - Status quo: components own the responsibility, document the contract clearly, ship a `html_escape::encode_text` helper.
- **Prose / Markdown paths.** Both already produce structured output. They need clear adapters to `BrowserFragment` — does Prose produce a `BrowserFragment` directly, or render to a string that then becomes the body?
- **Accessibility (ARIA, semantic HTML).** Not explicitly in scope; components are responsible. Should the spec at least include a "components SHOULD follow ARIA conventions" line?

---

## 5. CSS Variables: Per-Fragment or Page-Only?

The current draft puts the variables map on `HtmlPage` and `PageOptions`. The spec text says "we should inject CSS variables" but doesn't pin where they can be defined.

### Discussion points

- **Decision needed:** components *consume* variables via `var(--foo)`; only the **page** *declares* them. This makes theming tractable — a single source of truth.
- **What happens if a component emits a `:root { … }` block in its body string?** It would technically work (browsers honor it), but it breaks the model. Worth either (a) documenting this as anti-pattern, or (b) putting the variables emission entirely under page control (so anything emitted in a fragment body is structurally wrong, not just stylistically).
- **Per-component variable scopes?** Could a component declare its own scoped variables, e.g. `.simple-table { --row-pad: 4px; }`? This is fine — that's still a class-scoped variable, not page-level. Different mechanism.

---

## 6. `Layout::page_bg_color` in HTML Context

After the layout-move spec lands, `PageOptions.layout: Option<Layout>` carries `page_bg_color: Option<Color>`. What does that *mean* when rendering HTML?

### Discussion points

- **Where is the color applied?** `body { background-color: …; }`? `html { … }`? Wrapped in a `<style>` block at page level?
- **Translation from `Color`** (rich palette type) to a CSS color value. `Color::Rgb(RgbColor)` → `rgb(r, g, b)`; `Color::Web(WebColor::Tomato)` → `tomato`; `Color::Tailwind(Tailwind::Blue500)` → `var(--color-blue-500)` (assuming Tailwind palette is injected). Each variant has a natural target form.
- **What about `Color::DefaultBackground` and `Color::DefaultForeground`?** These only make sense in a terminal. Probably ignored in HTML output. Document that.
- **Do other `Layout` fields apply in HTML?** Margins, alignment, word-wrap — these *can* map to CSS (`margin`, `text-align`, `word-break`) but should they apply implicitly to the page body? Probably not — they describe the *content area*, not the page chrome. But this is fuzzy and worth deciding.

---

## 7. External Dependency Strategy

Components can declare `dependency_links: Vec<LinkTag>` (e.g. `MermaidDiagram` wanting `mermaid.css`). What does the page renderer do with them?

### Discussion points

- **Inline vs `<link>` strategy.** Options:
    1. Always emit `<link>` — simplest, but every page render fetches the external resource at view time.
    2. Always inline — page assembler fetches and bakes the contents into a `<style>` block. Bigger HTML, no extra requests.
    3. Hybrid policy chosen at render time via `PageOptions`.
- **Version conflicts.** If Component A wants `mermaid@9` and Component B wants `mermaid@10`, what happens? Most likely answer: last-write wins with a warning emitted at render time, but worth deciding the user-facing message and whether it should be hard-fail-able.
- **CDN vs local.** Is there a concept of "fetch at build time, inline" vs "leave as URL"? This may overlap with #11 below.

---

## 8. Two Paths to the Title

Currently, the page title can be set via:

- `HtmlPage.title: Option<String>` (direct field on the page).
- `BrowserFragment::add_metadata_keypair(MicrodataKey::Title, value)` (microdata, on a fragment).

### Discussion points

- **Pick one.** My recommendation: drop the direct field; route everything through microdata. The fan-out into HTML title + OG + Twitter + Schema.org is automatic and we get no benefit from a second code path.
- If the direct field is kept, what's the precedence rule? (Probably page-level overrides fragment-level, which matches the rest of `<head>` ordering.)

---

## 9. Inline-Style-vs-Class Overrides — Document the Reality

The spec says "the `style` will always override the default 'class' definition" and frames `!important` as a clean escape hatch.

### Discussion points

- **CSS specificity is richer than this framing suggests.** `#id .foo` beats inline `style` on most modifiers but not on the element bearing the style; `:where()` can demote selectors to zero specificity; `@layer` rules cascade in declared layer order; etc.
- **For the API surface, "instance style overrides class default" is fine as the typical case.** Worth rewording the spec to:

    > "Caller `style` values take precedence over component class defaults in typical use. CSS specificity ultimately decides; callers needing to defeat external high-specificity rules may need `!important`, which is not emitted by this library automatically."

- **Anti-pattern:** never emit `!important` from the renderer or builder API. Document this.

---

## 10. Naming: `HtmlStyleSheet` vs `Stylesheet`

`Stylesheet` (from darkmatter, soon `renderable`) = a single ruleset's declaration block.
`HtmlStyleSheet` = a collection of `(selector, Stylesheet)` rulesets.

Same word, different concepts at different levels of the hierarchy. Confusing.

### Discussion points

- **Rename candidates** for `HtmlStyleSheet`:
    - `CssRuleSet` — closest to actual CSS terminology, "a stylesheet is a list of rulesets".
    - `Rulesets` — short, plural makes it clear it's a collection.
    - `HtmlRules` — pairs nicely with `HtmlPage`.
    - `CssRules` — symmetric with browser DOM API (`CSSStyleSheet.cssRules`).
- I'd lean toward `CssRuleSet` (correctness) or `HtmlRules` (brevity) — pick the one you'll remember.

---

## 11. Anti-Pattern & Convention List for Spec

Things the spec should explicitly call out as **don't**s:

- Don't emit `!important` from builder APIs.
- Don't reference palette colors directly in component CSS — use semantic tokens via `var(--name)` once a token layer is decided.
- Don't define page-level `:root` CSS variables from a component.
- Don't return un-escaped user text in the `body` string.
- Don't depend on `<head>` element ordering beyond what the spec guarantees.
- Don't embed inline `<style>` or `<script>` in `body` — use the `code_features` / `stylesheet` / `script_blocks` mechanisms.

Most of these are conventions; do we want runtime checks (debug-mode assertions), lints, or just docs?

---

## 12. Migration Table for Existing Implementors

Once the new trait surface stabilizes, we'll need to migrate the four existing implementors. Sketch (to flesh out post-discussion):

| Component | Current `render_to_browser` shape | Migration |
|---|---|---|
| `HorizontalRule` | SVG string with embedded `var(--…)` placeholders | Move SVG generation into `body`; lift the `var(--…)` substitution out of `render_to_browser_with_inline_variables` per #2 above |
| `GraphExpression` | HTML/SVG output | Bridge default impl probably suffices; component has no scripts/dependencies |
| `YamlBlock` (darkmatter) | HTML output | Bridge default impl; if syntax highlighting is added it may need a stylesheet |
| `DarkmatterPage` (darkmatter) | Full page output | This one is the *page* assembler. May not need `render_html_fragment` at all — it's the consumer of fragments, not a producer. Discuss. |

---

## Suggested order for tomorrow

If we have time for a single session, prioritize:

1. **Composition / merge** (#1) — biggest blocker; everything downstream depends on the answer.
2. **`render_to_browser` migration story** (#2) — second-biggest; affects implementors immediately.
3. **CSS variables: semantic vs palette** (#3) — needed before we can lock down what gets emitted.
4. **Sanitization & body API** (#4) — small but easy to lock in.
5. **Naming** (#10) — quick fix, unblocks the rest of the writing.
6. The rest as time permits.

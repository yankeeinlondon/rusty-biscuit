# Review 2: Project 1 and Project 2 Implementation

## Findings

### High: Component metadata conflict resolution is backwards

- Location: `renderable/src/html/mod.rs:206`
- Spec source: `renderable/features/2026-05-14-kickoff/decisions.md:192`

`HtmlPage::merged_metadata` walks fragments in document order, but it uses `HashMap::insert` for every component metadata pair. That means later components overwrite earlier components for the same `MicrodataKey`.

The decision record says page-level metadata always wins, but component-vs-component conflicts are first-write wins in document order. The current implementation is last-write wins for component metadata.

Suggested fix:

- Change the component aggregation loop to preserve existing entries, for example `merged.entry(*key).or_insert_with(|| value.clone())`.
- Keep the later page-level loop as overwrite behavior.
- Add an integration test with two fragments that both set `MicrodataKey::Description`, asserting the first component description is rendered, then add page-level description and assert the page value wins.

### High: CSS variable layer does not ship the promised palette layer

- Location: `renderable/src/tokens.rs:60`, `renderable/src/html/mod.rs:343`, `renderable/src/color/color_enum.rs:61`
- Spec source: `renderable/features/2026-05-14-kickoff/decisions.md:73`

Decision 3A says `renderable` ships both a Tailwind-derived palette layer such as `--color-blue-500: #3b82f6` and a semantic layer whose defaults reference palette tokens. Decision 3D also says `Color::Var(String)` survives for palette tokens and arbitrary caller-defined variables.

The current implementation only emits semantic, spacing, and font variables from `root_defaults()`. Semantic color defaults are literal hex values, and `Color` has no `Var(String)` variant. As a result, components cannot rely on declared palette variables, and the documented semantic-to-palette relationship is not present.

Suggested fix:

- Add palette variable generation from `Tailwind` values and include it in the page `:root` block.
- Make semantic color defaults reference palette variables, for example `var(--color-blue-500)`, where that matches the intended token.
- Restore or add `Color::Var(String)` if the color API is expected to represent arbitrary CSS variables.
- Add tests asserting `page.stylesheet()` contains a representative palette token and that semantic color tokens reference `var(--color-...)` instead of hard-coded hex values.

### Medium: `render_html_page` silently drops invalid `PageOptions`

- Location: `renderable/src/browser/renderable.rs:66`, `renderable/src/html/mod.rs:137`
- Spec source: `renderable/features/2026-05-14-kickoff/decisions.md:146`

`HtmlPage::apply_page_options` correctly returns an error for absolute external asset paths, but the default `BrowserRenderable::render_html_page` ignores that result with `let _ = ...`. Passing an invalid `PageOptions` value therefore returns a page with the options silently omitted.

That weakens the Project 2 rule that external asset paths are enforced relative. It also makes the trait default behave differently from callers that use `HtmlPage::apply_page_options` directly.

Suggested fix:

- Decide on one explicit behavior for the trait default: make invalid options impossible, panic with a clear message, or change the API to return a `Result` before the surface settles.
- At minimum, add a test covering `render_html_page(Some(PageOptions { external_stylesheet: Some("/absolute".into()), .. }))` so the intended behavior is locked down.

### Medium: `ComposableNode::Component` public shape does not match the plan

- Location: `renderable/src/browser/fragment.rs:75`, `renderable/src/browser/fragment.rs:253`
- Spec source: `renderable/features/2026-05-14-kickoff/plan-project-2-new-api-surface.md:1766`

The Project 2 checklist says `ComposableNode` should include `Component(BrowserFragment<Ready>)`. The implementation uses `Component(Box<BrowserFragment<Ready>>)` to break recursive type size.

The boxing is understandable for Rust layout, but it changes the public constructor shape. Code written against the plan, for example `ComposableNode::Component(child_fragment)`, will not compile even though the documentation around `add_component` still describes that shape.

Suggested fix:

- Either adjust the public data model so the enum variant remains plan-compatible, for example by boxing the recursive field inside `BrowserFragment`, or update the feature plan and docs to explicitly bless `Component(Box<BrowserFragment<Ready>>)`.
- Add a public API test that exercises the intended construction path, not only the convenience `add_component` helper.

### Medium: Rustdoc validation is not clean

- Location: `renderable/src/stylesheet/mod.rs:7`, `renderable/src/stylesheet/error.rs:6`, `renderable/src/stylesheet/prop.rs:3`, `renderable/src/stylesheet/value.rs:3`, `renderable/src/html/tag/meta.rs:4`
- Spec source: `renderable/features/2026-05-14-kickoff/plan-project-2-new-api-surface.md:1766`

`cargo doc -p renderable --no-deps` succeeds, but it emits 37 warnings, mostly broken intra-doc links in the moved stylesheet modules plus invalid raw HTML tags in `meta.rs`.

The Project 2 verification checklist requires no new warnings and resolved intra-doc links. This is also relevant to Project 1 because stylesheet extraction moved the docs into `renderable`.

Suggested fix:

- Qualify cross-module links, for example `crate::stylesheet::CssStyle`, `crate::stylesheet::CssProp`, and `std::fmt::Display`.
- Escape literal HTML tag names in rustdoc, for example `` `<script>` ``.
- Consider running `RUSTDOCFLAGS="-D warnings" cargo doc -p renderable --no-deps` once the links are fixed.

## Test Coverage Gaps

- Add coverage for component metadata conflict ordering and page-level metadata override.
- Add coverage for the palette CSS variables and semantic token default values.
- Add coverage for invalid `PageOptions` through the `BrowserRenderable::render_html_page` default.
- Add a public API shape test for `ComposableNode::Component`, so the intended boxed-or-unboxed constructor is deliberate.

## Verification Run

- `cargo test -p renderable`: passed, including 50 unit tests, 17 integration tests, and 34 doctests.
- `cargo check -p biscuit-terminal`: passed.
- `cargo check -p darkmatter`: passed.
- `cargo doc -p renderable --no-deps`: completed with 37 rustdoc warnings.

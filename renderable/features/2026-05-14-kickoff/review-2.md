# Review 2: Project 1 and Project 2 Implementation

## Scope Note

`Color::Var(String)` is out of scope for this review. I did not treat its absence as a defect, even though older planning text still mentions it.

## Findings

### High: `BrowserRenderable::render_html_page` no longer matches the agreed API

- Location: `renderable/src/browser/renderable.rs:69`
- Spec source: `renderable/features/2026-05-14-kickoff/decisions.md:52`, `renderable/features/2026-05-14-kickoff/decisions.md:146`, `renderable/features/2026-05-14-kickoff/plan-project-2-new-api-surface.md:107`

The implemented trait method returns `Result<HtmlPage, PageOptionsError>`:

```rust
fn render_html_page(
    &self,
    page: Option<PageOptions>,
) -> Result<HtmlPage, PageOptionsError>
```

The decisions and Project 2 plan require `render_html_page(&self, page: Option<PageOptions>) -> HtmlPage`. They are explicit that this returns an `HtmlPage`, not a `String`, and the sample default returns the page directly.

The current implementation is defensible from an error-handling perspective because `HtmlPage::apply_page_options` can reject absolute external asset paths. The issue is that it changes the public trait contract during the coexistence window, which undermines the Project 2 goal of adding the new surface without burdening existing implementors and callers.

Suggested fix:

- Restore the trait signature to return `HtmlPage`, or update `decisions.md`, `spec.md`, and the Project 2 plan if `Result` is now the intended API.
- If the signature remains `HtmlPage`, avoid silent option loss by making `PageOptions` validate before construction, adding infallible relative-path wrapper types, or documenting a panic-on-invalid default.
- Add a focused API test that compiles against the intended return type so this does not drift again.

### Medium: `DarkmatterPage` still implements `BrowserRenderable`

- Location: `darkmatter/lib/src/layout/page.rs:728`
- Spec source: `renderable/features/2026-05-14-kickoff/decisions.md:255`

Decision 12A says `DarkmatterPage` is not a `BrowserRenderable`; it is a page assembler that consumes many fragments and produces an `HtmlPage`. The implementation still has:

```rust
impl BrowserRenderable for DarkmatterPage
```

That keeps the old role boundary in place. It also means `DarkmatterPage` inherits the component-oriented `render_html_fragment` / `render_html_page` defaults, which wrap the assembled page HTML as `RawHtml` even though the decision says a page assembler is not a component.

Suggested fix:

- Remove the `BrowserRenderable` impl for `DarkmatterPage`.
- Keep the inherent `render_to_browser(&self, md: &Markdown) -> Result<String, PageRenderError>` if it is still needed for compatibility.
- Add or adjust tests so page assembly is exercised through the intended `HtmlPage` builder path rather than through the component trait.

## Resolved Since Prior Review

- Component metadata now uses first-write-wins for component conflicts and page-level metadata still overrides.
- The CSS token layer now emits Tailwind palette variables and semantic color defaults reference the palette.
- `ComposableNode::Component` is now documented and planned as `Component(Box<BrowserFragment<Ready>>)` to account for recursive type layout.
- `cargo doc -p renderable --no-deps` is clean.

## Test Coverage Gaps

- Add coverage for the intended `render_html_page` return type and invalid `PageOptions` behavior once the API decision is settled.
- Add coverage ensuring `DarkmatterPage` is no longer used through `BrowserRenderable`; this may need a compile-fail/API-shape test rather than a runtime test.

## Verification Run

- `cargo test -p renderable`: passed, including 52 unit tests, 21 integration tests, and 34 doctests.
- `cargo doc -p renderable --no-deps`: passed with no warnings.
- `cargo check -p biscuit-terminal`: passed.
- `cargo check -p darkmatter`: passed.

# Project 3 Review

Reviewed against:

- `renderable/features/2026-05-14-kickoff/spec.md`
- `renderable/features/2026-05-14-kickoff/plan-project-3-migrate-callers.md`

## Summary

The core Project 3 migration is in good shape:

- `BrowserRenderable` no longer exposes `render_to_browser` or `render_to_browser_with_inline_variables`.
- The three migrated implementors use `render_html_fragment`.
- `DarkmatterPage` no longer implements `BrowserRenderable`; its inherent `render_to_browser(&Markdown)` API remains.
- `HtmlStyleSheet` has been renamed to the new `Stylesheet` collection type, and the declaration block is now `CssStyle`.
- `BrowserFragment<Ready>::render()` and `HtmlPage::render()` are implemented enough for the new page-composition example.

I found no clear broken Project 3 runtime behavior in the migrated API surface. The remaining issues are verification and coverage gaps.

## Findings

### Medium: `cargo test -p darkmatter` fails in doctests

`cargo build -p renderable -p biscuit-terminal -p darkmatter` passes, and `cargo test -p darkmatter --lib` passes, but the plan's fallback verification command `cargo test -p darkmatter` fails during the doctest phase.

Observed failures:

- `darkmatter/lib/src/diff/visual/side_by_side.rs:18`: rustdoc reports `can't find crate for biscuit_terminal`.
- `darkmatter/lib/src/mermaid/mod.rs:18`: rustdoc reports unresolved import `render_terminal::MermaidRenderError`.

This may be an existing rustdoc setup issue rather than a Project 3 regression, but Project 3's verification section requires all tests to pass. Until this is fixed or explicitly excluded, the project cannot be called fully verified with the fallback test runner.

Suggested fix:

- Make `cargo test -p darkmatter --doc` pass, or update the project verification instructions if darkmatter doctests are intentionally out of scope.
- For the Mermaid re-export, prefer an explicit relative path if rustdoc is mis-resolving it:
  `pub use self::render_terminal::MermaidRenderError;`
- Investigate why rustdoc cannot resolve `biscuit_terminal` even though normal builds and tests can.

### Low: Missing direct `render_html_fragment` coverage for `GraphExpression` and `YamlBlock`

Project 3's migration plan called for tests proving each migrated implementor's fragment path works. Current coverage checks the private/inherent helpers and `as_any`, but not the trait rendering path for two of the three implementors:

- `biscuit-terminal/lib/src/components/graph_expression.rs` tests `render_browser_svg()`, but not `GraphExpression::render_html_fragment().render()`.
- `darkmatter/lib/src/markdown/yaml_block.rs` tests `BrowserRenderable::as_any`, but not `YamlBlock::render_html_fragment().render()`.
- `HorizontalRule` is covered indirectly by `biscuit-terminal/lib/tests/html_page_example.rs`.

Suggested tests:

- Add a `GraphExpression` unit test that constructs a small graph, calls `render_html_fragment().render()`, and asserts the output is non-empty SVG and not escaped.
- Add a `YamlBlock` unit test that calls `render_html_fragment().render()` and asserts the rendered fragment includes the code block wrapper/body content.

This matters because the trait implementation could regress while the private helper tests keep passing.

### Low: Stale `HtmlStyleSheet` text keeps the verification grep non-zero

The code no longer defines or imports `HtmlStyleSheet`, but `renderable/src/stylesheet/sheet.rs:4` still says it "replaces the former `HtmlStyleSheet`". That makes this verification command from the plan return a hit:

```sh
rg -n "HtmlStyleSheet" renderable biscuit-terminal darkmatter --glob '*.rs'
```

Suggested fix:

- Remove the historical `HtmlStyleSheet` mention from the module docs, or adjust the verification command to ignore intentional historical notes.

## Verification Run

Commands run:

```sh
cargo build -p renderable -p biscuit-terminal -p darkmatter
cargo nextest run -p renderable -p biscuit-terminal -p darkmatter
cargo test -p renderable
cargo test -p biscuit-terminal
cargo test -p darkmatter
cargo test -p darkmatter --lib
cargo test -p biscuit-terminal --test html_page_example
rg -n "render_to_browser_with_inline_variables|fn render_to_browser\\b|impl BrowserRenderable for DarkmatterPage|HtmlStyleSheet" renderable biscuit-terminal darkmatter --glob '*.rs'
```

Results:

- Build passed.
- `cargo-nextest` is not installed in this environment.
- `cargo test -p renderable` passed.
- `cargo test -p biscuit-terminal` passed.
- `cargo test -p darkmatter --lib` passed.
- `cargo test -p biscuit-terminal --test html_page_example` passed.
- `cargo test -p darkmatter` failed in doctests as noted above.
- Deprecated trait method grep is clean; the only `fn render_to_browser` hit is the retained inherent `DarkmatterPage::render_to_browser(&self, md: &Markdown)`.

# Project 1 and 2 Implementation Review

## Findings

### 1. Project 1 did not finish the `darkmatter` stylesheet extraction

Project 1 requires stylesheet data and target-agnostic emitters to move to `renderable`, with `darkmatter` consumers importing the moved types from `renderable::stylesheet` and terminal-only behavior restored through `biscuit-terminal` ([plan-project-1-stable-base.md:104](plan-project-1-stable-base.md#phase-4-extract-stylesheet-data-to-renderable), [plan-project-1-stable-base.md:117](plan-project-1-stable-base.md#phase-4-extract-stylesheet-data-to-renderable)). The implementation added the new `renderable::stylesheet` module, but `darkmatter` still owns and exports a full independent stylesheet system:

- [darkmatter/lib/src/render/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/render/mod.rs:9) still declares `pub mod stylesheet`.
- [darkmatter/lib/src/render/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/render/mod.rs:15) still re-exports `CssColor`, `CssProp`, `CssUnit`, `Stylesheet`, and `StylesheetError` from that local module.
- [darkmatter/lib/src/render/image_ref.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/render/image_ref.rs:16), [darkmatter/lib/src/render/link.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/render/link.rs:18), and [darkmatter/lib/src/markdown/errors/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/errors/mod.rs:34) still consume `crate::render::stylesheet`.
- [darkmatter/lib/src/render/stylesheet.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/render/stylesheet.rs:2086) still defines its own `Stylesheet`, while `renderable` now defines `CssStyle` / `Stylesheet` separately.

Impact: `renderable` is not the shared source of truth for stylesheet data. The two crates now have divergent CSS types, downstream code cannot pass darkmatter styles into the new page API without conversion, and the Project 1 compatibility/error-downcast work was not actually validated against the moved types.

Suggested fix: migrate `darkmatter` consumers to `renderable::stylesheet`, keep only darkmatter-specific terminal presentation/error wrapping locally, and add a darkmatter compatibility test that exercises `ImageRef`/`Link` with the moved `StylesheetError`.

### 2. Nested component auxiliary state is silently lost during page assembly

The decisions document says `HtmlPage` must walk the fragment tree, descend through `ComposableNode::Component`, and collect stylesheets, features, metadata, and dependency links ([decisions.md:33](decisions.md#1-composition-model)). The current implementation only reads auxiliary state from top-level page fragments:

- `collect_dedup_links()` loops over `self.fragments` and each fragment's direct `dependency_links()` only ([html/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/mod.rs:149)).
- `stylesheet()` only includes stylesheets attached to direct page fragments ([html/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/mod.rs:172)).
- `render()` uses only page-level `self.metadata` for `<title>` and meta tags; fragment metadata is never merged at all ([html/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/mod.rs:199)).
- The `features` field is stored but never rolled up or used ([html/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/mod.rs:39)).

Impact: a component composed inside another component can render its body HTML, but its stylesheet, dependency links, metadata, and features disappear. This is exactly the "silent data-loss foot-gun" the Project 2 design was trying to avoid.

Suggested fix: add recursive collectors that visit every `BrowserFragment<Ready>` and descend through `ComposableNode::Component`. Use those collectors for links, stylesheet rollup, metadata rollup with page-level override precedence, and feature rollup. Add tests where a nested child component contributes a stylesheet, dependency link, metadata title/description, and feature.

### 3. Fragment rendering drops the wrapper class and most typed attributes

`define_as_block_tag(tag, base_class)` stores the base class on `HtmlBlockTag` ([fragment.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/browser/fragment.rs:140), [tag/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/tag/mod.rs:399)), and `ComponentStylesheet::as_stylesheet()` scopes rules under that wrapper class ([browser/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/browser/mod.rs:37)). But the renderer never emits `base_class`, so scoped CSS such as `.simple-table .col-string` cannot match the component root. The renderer also intentionally drops `Class`, `Id`, `Style`, boolean attributes, ARIA, `data-*`, `rel`, `type`, and `value` in `render_attributes()` ([fragment.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/browser/fragment.rs:337)).

Impact: the new typed fragment API can build structurally valid nodes, but common browser output is behaviorally incomplete. Styling does not attach to roots, inline styles cannot render, ARIA/data attributes disappear, and typed link/form/image attributes beyond the small allowlist are lost.

Suggested fix: include the block's `base_class` in the rendered `class` attribute, merge it with any explicit `HtmlAttribute::Class`, and serialize every `HtmlAttribute` variant that has a defined HTML representation. Add tests for base class emission, class merging, id/style, boolean attributes, ARIA/data attributes, and escaping of each string-valued attribute.

### 4. `ComponentStylesheet` cannot be populated through the public API

`ComponentStylesheet` exposes `new()`, `name()`, and `as_stylesheet()` only ([browser/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/browser/mod.rs:24)). Its `style` field is private ([browser/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/browser/mod.rs:20)), and no `add`/`push` method exists. The supporting design docs explicitly refer to `ComponentStylesheet::add` as the way component authors register class-scoped rules ([browser-utils.md:132](browser-utils.md#css_slug)).

Impact: callers can attach an empty component stylesheet but cannot build a useful one without crate-internal access. This also leaves `ComponentStylesheet::as_stylesheet()` effectively untested for real entries.

Suggested fix: add a public `add(selector, CssStyle) -> Self` or `push(selector, CssStyle) -> &mut Self` API, matching the surrounding builder style. Add tests that create a component stylesheet, add an internal class rule, attach it to a fragment, and assert the page CSS contains the scoped descendant selector.

### 5. External asset paths are documented as relative but not enforced or escaped

`PageOptions` documents `external_stylesheet` and `external_code` as "enforced relative" ([browser/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/browser/mod.rs:80)), matching the decision that paths must be relative for portable `href`/`src` values ([decisions.md:174](decisions.md#7-external-dependency-strategy)). The implementation accepts any `PathBuf` and inserts `path.display()` directly into HTML attributes ([html/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/mod.rs:226), [html/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/html/mod.rs:239)).

Impact: absolute paths are accepted despite the contract, and paths containing quotes or other HTML-significant characters can produce invalid markup. The existing test only covers the happy path with `assets/page.css` ([render_pipeline.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/tests/render_pipeline.rs:79)).

Suggested fix: make `PageOptions` construction validate relative paths, or make `apply_page_options` return a `Result` and reject absolute paths. Escape `href`/`src` values during render. Add negative tests for absolute paths and special-character paths.

## Test Coverage Gaps

- Add recursive page assembly tests for nested components. Current tests cover top-level fragment rendering and page basics only ([render_pipeline.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/tests/render_pipeline.rs:6)).
- Add a compile/runtime test for the default `BrowserRenderable::render_html_fragment()` and `render_html_page()` shims so the coexistence surface stays intact for legacy implementors.
- Add darkmatter tests after stylesheet migration to prove `ImageRef`, `Link`, YAML/browser output, and stylesheet error downcasts use `renderable::stylesheet`.
- Add attribute serialization tests for all `HtmlAttribute` variants that are expected to emit HTML.
- Add tests proving component stylesheet rules actually affect emitted markup: root wrapper class present, scoped selector present, and nested component styles rolled up.

## Verification

- `cargo test -p renderable` passes: 50 unit tests, 7 integration tests, and 32 doctests passed.
- `cargo metadata --no-deps --format-version 1` confirms package names `renderable`, `biscuit-terminal`, `biscuit-terminal-cli`, and `darkmatter`.
- `cargo tree -p renderable` shows no `biscuit-terminal` or `darkmatter` dependency under `renderable`.

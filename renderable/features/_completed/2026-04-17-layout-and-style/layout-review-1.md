# Review 1

## Findings

### P1 - Terminal tree layout ignores `max_width`

`Writer::render_with_layout` resolves the four margins and then narrows child rendering only with `available.saturating_sub(left + right).max(1)` (`biscuit-terminal/lib/src/render_tree/render.rs:172-181`). It never resolves or applies `layout.max_width`, so terminal output can still occupy the full margin-adjusted width even when a block declares a cap. This also breaks the spec's composition rule because children receive a width reduced by margins only, not by the parent's `max_width`, and it makes alignment mostly unobservable for the intended `max_width` case.

The spec requires terminal renderers to apply `margin` / `max_width`, pass the reduced inner width to children, and test alignment with and without `max_width`. Please resolve `layout.max_width` for `RenderTarget::Terminal`, cap `content_width` before rendering the subtree, and add coverage for nested layout composition and alignment under the cap.

### P1 - Invalid `Layout` values are not rejected by the render-tree path

`Layout::validate` exists (`renderable/src/layout/mod.rs:73-78`), but neither `NodeAttrs::set_layout` / `NodeAttrs::layout` (`renderable/src/tree/attrs.rs:1152-1174`) nor tree validation calls it. The renderers then consume the layout directly: browser lowers every resolved length to CSS (`renderable/src/tree/render/browser.rs:733-801`), while terminal resolves CSS units to `0` and casts arbitrary percent values (`biscuit-terminal/lib/src/render_tree/render.rs:996-1007`).

This means a node can carry `TargetValue::Universal(Length::Css(...))`, an empty `PerTarget` map, or `Length::Percent(200.0)` and still render instead of producing the required actionable validation error. In particular, the required "invalid universal units" test would currently fail if it goes through a real render-tree render rather than calling `TargetValue::validate` directly. Please integrate `Layout::validate` into render-tree validation for layout-bearing nodes and ensure all renderers fail before lowering invalid values.

### P2 - Inline-node layout does not honor strictness policy

The block-only rule currently records layout on inline nodes as an error unconditionally (`renderable/src/tree/validate.rs:316-322`). Both browser and terminal gates reject any validation error before checking renderer strictness, so `Warn` and `Lossy` cannot ignore the layout and continue with a diagnostic as required by D5.

The spec says this should fail only under `RenderStrictness::Strict`; under `Warn` / `Lossy`, the layout should be ignored and a diagnostic recorded. Please represent this rule as a warning-severity validation finding, or otherwise make it strictness-aware before the hard validation gate, and ensure the renderer drops the inline layout instead of applying it.

### P2 - `DarkmatterPage` still owns the old page-layout contract

The darkmatter migration is still mostly a compatibility layer rather than the spec's target architecture. `DarkmatterPage` continues to store `PageMargin`, `PagePadding`, `PageAlignment`, and `PageFill` as its internal state (`darkmatter/lib/src/layout/page.rs:58-72`), and `LayoutContext` is explicitly derived from those deprecated types (`darkmatter/lib/src/layout/context.rs:1-12`). The old types are deprecated and conversion helpers exist, but page-level margins have not become a `renderable::layout::Layout` on the document root, and the page assembler still reimplements layout decisions.

That falls short of D9 and the success criterion that `DarkmatterPage` stop re-inventing the layout contract. Please either complete the migration so the page path is driven by `renderable::layout::Layout`, or document this as a deliberate deferral with tests proving the deprecation bridge is the accepted compatibility boundary for Spec A.

## Verification

Ran `cargo test -p renderable -p biscuit-terminal -p darkmatter --quiet`. The run failed in `darkmatter` on `markdown::compose::frontmatter_shell_expansion::execution_tests::execute_frontmatter_commands_concurrently`: expected under `650ms`, got `678.571375ms`. Earlier test batches for `renderable` / `biscuit-terminal` passed before that timing-sensitive `darkmatter` failure.

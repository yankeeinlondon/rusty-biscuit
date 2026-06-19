---
agent: codex
model: ""
ready: false
---

# Review: Disclosure Blocks

## Findings

### High: Terminal disclosure body is not rendered dim + italic

The spec requires terminal output to render the disclosed body as a block quote whose text is dim and italic. The implementation attempts to apply a dim/italic inherited style in `biscuit-terminal/lib/src/render_tree/render.rs::render_disclosure`, but the focused test currently fails because the rendered output has no dim or italic SGR escapes:

```text
cargo test -p darkmatter --test disclosure_render_targets --color=never
...
test terminal_target_renders_summary_and_dim_italic_body ... FAILED
body must contain dim escape: License Agreement

│  Keep your hands off.
```

This is a functionality gap, not only a coverage gap. The body is shown and quoted, but the target-specific visual distinction required by the spec is missing.

Verification level present: Level 1, and failing. Once fixed, this should also have Level 2 coverage because the requirement is terminal-rendered SGR/glyph behavior through a real terminal capture.

### High: `style.disclosure.color` and `style.disclosure.bg-color` parse but are not applied

`apply_disclosure_style` only delegates to `apply_common_style`, which lowers width, max-width, and alignment into the `PageComponent::Disclosure` policy. Disclosure is missing from `apply_color_style`, so frontmatter `style.disclosure.color` and `style.disclosure.bg-color` never reach `with_component_color` / `with_component_bg_color`.

Relevant code:

- `darkmatter/lib/src/style/apply.rs:706` calls `apply_common_style` for disclosure.
- `darkmatter/lib/src/style/apply.rs:780` only applies layout/alignment.
- `darkmatter/lib/src/style/apply.rs:506` applies colors for table/images/block-quote/hyperlinks/lists, but not disclosure.

This violates the style bucket requirement for visible foreground/background paint. Existing tests prove the keys are parsed and not marked inactive, but they do not verify that the colors reach rendering.

Verification level present: Level 1 parser/coverage only. Needed: Level 1 policy assertions plus Level 2 terminal capture for terminal color styling; browser/html can be covered with HTML/style assertions unless computed style behavior is added.

### High: `md compose --output markdown-plus` is accepted but does not use the MarkdownPlus fold

The spec says `--output markdown-plus` maps to `OutputFormat::MarkdownPlus` and routes through the MarkdownPlus fold, emitting `<details><summary>...</summary>...</details>`. The top-level render path does this, but the compose path groups `OutputFormat::MarkdownPlus` with `Auto | Markdown` and prints composed Markdown unchanged.

Relevant code: `darkmatter/cli/src/commands.rs:1142`.

That means `md compose --output markdown-plus` preserves the DSL instead of rendering MarkdownPlus inline HTML, despite accepting the output value. Either route composed content through `markdown_plus_artifact` or reject `markdown-plus` for compose; accepting it as a synonym for Markdown contradicts the advertised output contract.

Verification level present: none found for compose + MarkdownPlus. Needed: Level 1 CLI test that composes a disclosure and asserts `<details>` / `<summary>` output for `md compose --output markdown-plus`.

### High: Terminal disclosure rendering lacks the required real-terminal verification level

The only terminal disclosure test found is `darkmatter/lib/tests/disclosure_render_targets.rs:110`, which calls `Markdown::as_terminal` in-process and checks raw bytes. Per the review instructions, terminal-rendered styling/glyph requirements need Level 2 when the behavior is user-observable terminal rendering. This feature includes specific block quote presentation and dim/italic body styling, so Level 1 alone is not enough to mark the target production-ready.

Needed: a `level2_*` real-terminal capture test, run through the package `just test-l2` flow, that verifies the summary/body text, quote prefix/width behavior, and terminal styling survive rendering through the selected terminal harness.

## Other Notes

- The render-tree and JSON shape are generally on the intended path: `Markdown::as_document()` now folds through the disclosure-aware render-tree entry point, and JSON serialization preserves native disclosure nodes.
- Transclusion unification has reasonable Level 1 integration coverage for `::file` and `::code`, including `disclosure=true` default summary behavior.
- `darkmatter/lib/tests/disclosure_render_targets.rs` contains a committed `debug_nested_fixture_events` test that prints the full serialized tree. It is not a correctness blocker, but it should be removed or converted into a real assertion-only regression test.

## Production Readiness

Not ready for production. The terminal target currently fails a specified behavior, disclosure frontmatter colors are parsed but not applied, compose accepts `markdown-plus` without producing MarkdownPlus output, and the terminal presentation lacks the required Level 2 verification.

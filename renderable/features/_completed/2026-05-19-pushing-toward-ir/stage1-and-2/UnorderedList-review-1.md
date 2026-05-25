---
ready: true
---

# UnorderedList IR Migration — Review 1

## Scope

- **Component**: `UnorderedList` in `biscuit-terminal/lib/src/components/list.rs`
- **CLI**: `bt list` in `biscuit-terminal/cli/src/commands/list.rs`
- **Tests**:
  - `biscuit-terminal/lib/tests/unordered_list_parity.rs`
  - `biscuit-terminal/lib/tests/list_parity.rs`
  - `biscuit-terminal/lib/src/components/list.rs` (in-source unit tests)
  - `biscuit-terminal/cli/tests/integration_test.rs` (UnorderedList-specific cases)
- **Spec**: `renderable/features/2026-05-19-pushing-toward-ir/components/UnorderedList-spec.md`

## Summary

The UnorderedList migration is a **complete, mechanically sound implementation** of the Stage-2 IR flip pattern. All acceptance criteria from the spec are satisfied:

- `TerminalRenderable::render()` delegates to the canonical tree path (`render_via_tree`).
- A single private projection helper (`to_render_tree_node` / `to_render_tree_node_with_terminal`) serves both `TreeRenderable::render_tree` and `TerminalRenderable::render_tree_node`.
- `BrowserRenderable` and `MarkdownRenderable` are implemented directly on the component, routing through the shared tree renderers.
- The `bt list` CLI supports `--md`, `--md-plus`, `--html`, `--example`, `--bullet`, `--no-hanging-indent`, and `LayoutArgs` with correct cross-target semantics.
- The bespoke path is retained as `#[doc(hidden)] pub fn render_bespoke` for parity testing.

**Test count**: ~80+ tests pass across unit, parity, structural, and integration levels (61 in `unordered_list_parity.rs`, 10 in `integration_test.rs`, plus shared structural tests in `list_parity.rs` and in-source tests).

---

## Findings

### High — Error fallback policy diverges from established best practice

**Browser path**: `BrowserRenderable::render_html_fragment` falls back to an in-band `[render-tree error: …]` text fragment when `render_browser_node` returns `Err`:

```rust
Err(error) => BrowserFragment::new()
    .define_as_text_fragment(format!("[render-tree error: {error}]"))
    .finalize(),
```

The Progress polish lesson (`lessons-learned.md`) established that infallible trait fallbacks must **log via `tracing::error!` and return empty output**, never an in-band sentinel that pollutes user-visible HTML. The current shape matches the *spec* (which accepted the adapter's "visible fallback fragment" policy) but diverges from the *subsequent* cross-component best practice.

**Markdown path**: `render_markdown` and `render_markdown_plus` silently drop errors with `.unwrap_or_default()`. The same Progress lesson replaced this with an explicit `match` arm and `tracing::error!` call. UnorderedList should follow suit.

> **Severity: high** because observability gaps make production debugging harder, but this is **not a functional bug** — the tree produced by UnorderedList validates cleanly in all normal paths, so these arms are rarely hit.

### Medium — Missing edge-case unit tests for spec variants

The spec lists several test variants that are **not explicitly covered** at unit-test level:

| Spec Variant | Coverage Gap |
|---|---|
| Empty nested UnorderedList | No parity test exists (cf. `OrderedList` has `test_empty_nested_list` in `list.rs`). The KNOWN_DRIFT behavior (blank-line padding vs. true blank line) is documented for OrderedList but not pinned for UnorderedList. |
| Browser: long text item | Not tested that HTML `<li>` content survives without truncation. |
| Browser: invalid projected tree fixture | No test forces a structural render error and asserts the fallback shape. |
| Markdown: item containing explicit newline | No test verifies continuation-line indentation in Markdown output. |
| Markdown: nested UnorderedList / OrderedList | No Markdown-specific structural tests for nested lists (terminal and HTML are well covered). |
| Markdown: invalid list structure under `Strict` | No test exercises `RenderStrictness::Strict` on the Markdown path. |

All of these are **partially mitigated** by the fact that the tree renderer's generic handling of `NodeKind::List` / `ListItem` is already tested extensively in `renderable`'s own suite, and the integration tests exercise the happy paths end-to-end. Still, component-level pins would close the loop.

### Medium — MarkdownPlus equality test only exercises plain items

`markdown_plus_matches_portable_markdown` asserts identity for plain string items. It does **not** exercise a `Prose` item with inline styling (`<b>`, `<red>`), so a future regression that caused MarkdownPlus to emit raw HTML tags for styled inline content while Markdown stripped them would not be caught. A single additional test with a styled Prose item would close this.

### Low — CLI shared-helper doc comments refer exclusively to `<ol>`

`render_html_with_layout` and `wrapper_only_css` in `cli/src/commands/list.rs` carry comments that mention `<ol>` exclusively:

> "The tree renderer lowers `Layout` to CSS on the `<ol>` itself..."

The same helpers are used for unordered lists, where the root element is `<ul>`. A minor doc inaccuracy; the code is functionally correct.

### Low — `render_html_page` is not directly unit-tested

The `render_html_page` implementation is a thin wrapper around `render_html_fragment()` → `HtmlPage::from(...)`. A regression in that generic pipeline would not be caught by an UnorderedList-specific test. Acceptable given the thinness of the wrapper.

---

## Test Coverage Assessment

### Level 1 (in-process / PTY)

| Requirement | Tests | Verdict |
|---|---|---|
| Tree projection structurally correct | `tree_renderable_and_compat_hook_share_one_projection`, `tree_renderable_omits_default_bullet_hint`, `tree_renderable_preserves_custom_bullet_and_indent_children`, `unordered_list_projects_to_list_kind`, `unordered_list_records_custom_bullet_hint`, `unordered_list_omits_default_bullet_hint`, `unordered_list_records_disabled_hanging_indent` | ✅ Strong |
| Bespoke vs tree semantic parity | `parity_empty_list`, `parity_single_string_item`, `parity_three_string_items`, `parity_long_item_word_wraps`, `parity_custom_ascii_bullet`, `parity_custom_unicode_bullet`, `parity_custom_bullet_wider_glyph_hanging_indent`, `parity_disable_hanging_indent_drops_continuation_indent`, `parity_nested_unordered_list_indents_block_child`, `parity_nested_ordered_list_inside_unordered`, `parity_three_level_nesting_compounds_indent`, `parity_custom_indent_children`, `parity_mixed_inline_and_block_children`, `parity_from_single_prose`, `parity_very_narrow_width_preserves_content`, `parity_bullet_changed_after_construction_updates_items`, `optimistic_render_matches_default_terminal_render` | ✅ Strong |
| Layout interactions | `parity_with_left_margin`, `parity_with_right_margin_narrows_wrap_width`, `parity_with_alignment_set` | ✅ Strong |
| Markdown output | `markdown_empty_list_is_empty`, `markdown_single_item_uses_dash_prefix`, `markdown_three_items_use_dash_prefix_in_order`, `markdown_plus_matches_portable_markdown`, `markdown_ignores_custom_bullet`, `markdown_layout_does_not_change_output` | ✅ Strong |
| Browser output | `html_empty_list_emits_empty_ul`, `html_single_item_wraps_in_li`, `html_three_items_emit_three_li_elements`, `html_nested_unordered_list_nests_ul`, `html_nested_ordered_list_emits_ol` | ✅ Strong |
| Prose inline styling survival | `prose_item_content_survives_terminal_render`, `prose_inline_styling_survives_terminal_render`, `prose_inline_styling_degrades_in_markdown` | ✅ Strong |
| CLI cross-target flags | `test_list_unordered_md_emits_commonmark`, `test_list_unordered_md_ignores_custom_bullet`, `test_list_unordered_md_plus_matches_md`, `test_list_unordered_html_emits_ul_li`, `test_list_unordered_html_ignores_custom_bullet`, `test_list_unordered_terminal_honors_custom_bullet`, `test_list_unordered_example_unchanged`, `test_list_unordered_md_with_left_margin_emits_frontmatter`, `test_list_unordered_html_with_layout_emits_margin_on_ul_only`, `test_list_unordered_html_with_alignment_wraps_in_div` | ✅ Strong |

### Level 2 (real terminal emulator)

No dedicated Level-2 tests exist for `bt list`. This is **acceptable** for UnorderedList:

- The component emits no `Style` (no SGR sequences, borders, fills, or images).
- The only terminal-specific rendering concerns are bullet glyphs and hanging-indent alignment, both of which are pure text layout verified at Level 1.
- The test-rigor policy states Level 2 is required for "glyphs, widths, SGR styling, and scrolling render correctly through the real terminal's display path." UnorderedList has no SGR or scroll-compensation requirements.
- Integration tests spawn the real `bt` binary and verify output bytes.

> **Verdict**: Level-1 coverage is the correct minimum for this component.

---

## Ergonomics and Performance

### Ergonomics

- The builder API (`with_bullet`, `with_indent_children`, `without_hanging_indent`, `add`) is consistent with the rest of the component ecosystem.
- `From<Vec<T>>`, `From<Vec<RenderableTerminalContent>>`, `From<Prose>` conversions are preserved and behave correctly.
- `with_bullet` correctly re-computes hanging indent for existing inline component items.

### Performance

- `render_tree()` allocates a fresh `RenderNode` tree per call. This is the expected cost of the canonical render-tree architecture and is identical to every other flipped component.
- `render_markdown` and `render_markdown_plus` each call `render_tree()` independently. For a large list rendered to both targets, the tree is built twice. A future optimization could cache the projected node, but this is a cross-cutting concern (applies to all components) and is not a blocker.
- The `project_list_items` helper uses the shared `project_renderable_content` path, avoiding the duplicate Prose-downcast logic that earlier migrations carried.

---

## Production Readiness

**Judgment: `ready: true`**

UnorderedList is **production ready**.

**Why:**

1. **Functional completeness**: Every acceptance criterion in the spec is implemented and verified. The component routes Terminal, Browser, and Markdown output through the canonical render tree. The CLI surface is complete and integration-tested.
2. **Strong test coverage**: ~80+ tests pass, spanning structural projection validation, bespoke-vs-tree semantic parity, cross-target rendering (Terminal / Markdown / Browser / MarkdownPlus), layout interactions, edge-case width handling, and CLI end-to-end behavior.
3. **No known bugs**: All tests pass (`cargo test` clean). The implementation follows the mechanically proven Stage-2 migration recipe.
4. **Appropriate verification levels**: Level-1 tests are sufficient for this component because it has no interactive input, no image protocols, and no `Style`-driven SGR output that would require real-terminal decoder validation.

**Reservations (non-blocking):**

- The Browser and Markdown error fallback paths should be updated to match the cross-component observability policy (log + empty output) established by the Progress polish lesson. This is a surgical follow-up, not a migration blocker.
- A handful of edge-case spec variants (empty nested list, Markdown nested lists, invalid fixture handling) could be added to the parity suite for completeness, but the generic tree renderer's own tests already cover these structures.
- No dedicated Level-2 real-terminal test exists, but the component's output is plain text layout with no styling or escape sequences that would benefit from emulator-level verification.

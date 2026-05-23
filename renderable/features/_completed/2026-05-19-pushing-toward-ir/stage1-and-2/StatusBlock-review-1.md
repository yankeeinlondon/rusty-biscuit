---
ready: true
---

# StatusBlock Implementation Review

**Review date:** 2026-05-20
**Spec:** `renderable/features/2026-05-19-pushing-toward-ir/components/StatusBlock-spec.md`
**Implementation:** `biscuit-terminal/lib/src/components/status_block.rs`
**Parity tests:** `biscuit-terminal/lib/tests/status_block_parity.rs`
**CLI:** `biscuit-terminal/cli/src/commands/status_block.rs`
**Integration tests:** `biscuit-terminal/cli/tests/integration_test.rs` (status-block section)

---

## Executive Summary

The StatusBlock IR migration is **well-executed** and follows the established Stage 2 recipe. The single private projection helper (`to_render_node`) is clean, the terminal/browser/markdown paths are all wired, the bespoke compatibility fallback for arbitrary borders is narrow and correctly isolated, and the test suite is comprehensive at Level 1. All tests pass.

The implementation matches the evolved codebase pattern rather than the literal spec sketch in a few places (notably `BrowserRenderable` calls `render_browser_node` directly rather than through `BrowserTreeComponent`, which is the same shape every other flipped component uses). These are documentation/spec-drift issues, not implementation gaps.

---

## What Was Reviewed

- `status_block.rs` — component implementation, trait impls, unit tests
- `status_block_parity.rs` — structural assertions + bespoke-vs-tree parity
- `status_block.rs` CLI command — arg parsing, target dispatch, example mode
- `integration_test.rs` — CLI integration tests for the `bt status-block` subcommand
- `renderable/docs/components.md` — component capability table
- `renderable/docs/tree-rendering.md` and `layout-and-style.md` — architectural context
- `lessons-learned.md` — migration precedent and pitfalls

---

## Findings

### 1. Missing `render_tree_node` compatibility hook — Low

**Finding:** `StatusBlock` does not override `TerminalRenderable::render_tree_node()`. The default method returns `None`, so if a `StatusBlock` is ever nested inside another component's `RenderableTerminalContent`, the generic projector falls back to `render_bespoke(term)` → ANSI strip → plain `Text`. The structural `BlockQuote` / `Paragraph` shape is lost.

**Impact:** Low. StatusBlock is a top-level error-reporting surface; nesting it inside a `Compose` or `TwoColumn` is not a documented use case. However, the Stage 2 recipe explicitly calls for both `TreeRenderable::render_tree()` and `TerminalRenderable::render_tree_node()` to delegate to the same helper. `BlockQuote` (the precedent component) also omits this hook, so the deviation is at least consistent.

**Remediation:** Add a `render_tree_node` override that returns `Some(self.to_render_node())`. This is a one-line change and costs nothing.

### 2. No dedicated Level-2 real-terminal test for `bt status-block` — Medium

**Finding:** The `level2_render_tree_style.rs` suite exercises `bt block`, `bt progress`, and `bt table` inside WezTerm, Kitty, and tmux. The BlockQuote border primitive is Level-2 tested via `bt quote`. However, there is **no Level-2 test that drives `bt status-block` itself** through a real terminal emulator.

**Impact:** Medium. The underlying primitives (`Style::Border` thick left border, `Style::color` foreground, layout margins) are all Level-2 verified via other commands, so the renderer's ability to lower them is proven. What is *not* proven is that StatusBlock's specific composition (root layout + header paragraph + block-quote body + hint paragraph) renders correctly in a real terminal — for example, that the root layout margins and the BlockQuote border do not interact badly.

**Remediation:** Add a `bt status-block` Level-2 case to `level2_render_tree_style.rs` (or a dedicated `level2_status_block.rs`) that asserts:
- The thick left border glyph `┃` is visible in captured cells.
- The header text and fallback icon are visible.
- Color SGR escapes survive on color-capable terminals.
- No raw escape garbage leaks into tmux plain capture.

### 3. `renderable/docs/components.md` table is stale — High (documentation)

**Finding:** The components table in `renderable/docs/components.md` still lists StatusBlock as:

| Component | Block | ✅ | ❌ | ❌ | ❌ | no changes | — | ... |

It should read:

| Component | Block | ✅ | ✅ | ✅ | ✅ | tree default + bespoke compatibility fallback | tree | ... |

**Impact:** High for documentation accuracy, low for runtime behavior. The table is the canonical public reference for which components support which targets. A reader relying on it will incorrectly believe StatusBlock is terminal-only.

**Remediation:** Update the StatusBlock row in `renderable/docs/components.md`.

### 4. Unit test does not assert border color value in projected tree — Low

**Finding:** `body_style_carries_thick_left_border` checks `border.weight`, `border.line_style`, and `border.sides`, but it does **not** assert that `border.color` equals the resolved severity color (or custom override). The parity test `custom_border_color_emits_sgr_on_color_terminal` checks that *some* SGR is emitted, but it does not verify the tree structure carries the correct color.

**Impact:** Low. The parity gate would catch a total color loss, but a subtle color swap (e.g., severity default leaking past a custom override) could regress silently.

**Remediation:** Add a unit test that asserts the serialized `NodeAttrs` JSON contains the expected color value (e.g., `Tailwind::Purple700` hex) when a custom `border_color` is set.

### 5. `render_html_page` not tested with actual `PageOptions` — Low

**Finding:** The unit test `html_page_includes_fragment` calls `render_html_page(None)`. The spec requires testing `render_html_page(None)`, which is satisfied, but there is no test exercising `render_html_page(Some(...))` with actual page options.

**Impact:** Low. The method body is a trivial delegation to `HtmlPage::from(fragment).apply_page_options(options)`, shared by every other component. A regression here would affect all components simultaneously and would be caught by any component that does test the `Some` path.

**Remediation:** Add a test that passes `Some(PageOptions::default())` and asserts the returned HTML contains expected page wrappers.

### 6. `prose_plain_text` allocates a Terminal per call — Low (performance)

**Finding:** `prose_plain_text` builds a fresh `Terminal` via the builder pattern, renders the `Prose`, and strips ANSI. For a StatusBlock with N body items, this happens N+2 times (header, each body item, hint). Each builder call allocates a small struct; for typical N ≤ 3 this is negligible.

**Impact:** Low. No measurable regression for normal usage.

**Remediation:** None required. If profiling ever shows this as hot, a cached no-color terminal could be reused.

---

## Ergonomics & Performance Observations

- **Single projection helper:** `to_render_node` is the sole source of truth. Both `TreeRenderable::render_tree` and `TerminalRenderable::render` (via `render_via_tree`) delegate to it. This is exactly the recipe and prevents drift.
- **Consistent error policy:** All three targets (Terminal, Browser, Markdown) log via `tracing::error!` and fall back to empty output on render failure. This matches the infallible trait contracts and avoids in-band sentinel pollution.
- **Icon ownership:** StatusBlock owns its portable icon table rather than reusing the `Status` component's `FB_*` constants. This prevents Nerd Font leakage into Markdown/Browser output, and the parity test correctly drops icons before token comparison.
- **Custom border isolation:** The arbitrary `border(String)` compatibility knob is cleanly gated to the bespoke terminal path. Regression tests explicitly prove it never leaks into Markdown or HTML.
- **Default layout seeded on root:** The comment explaining why `root.attrs.set_layout(&self.layout)` is required is load-bearing and correct — the adapters do not apply `tree_layout()`.

---

## Test Coverage Summary

| Category | Count | Verdict |
|----------|-------|---------|
| Unit tests (lib) | 43 | Strong. Covers tree structure, classes, style, layout, icon mapping, prose flattening, terminal rendering, markdown, browser, clone/debug, and bespoke fallback. |
| Parity tests | 16 | Strong. Covers structural validation, token parity across widths, color SGR, layout margins, custom border isolation, and every non-deprecated severity. |
| CLI integration tests | 9 | Strong. Covers `--example`, `--md`, `--html`, `--severity`, `--border-color`, `--help`, mutual exclusion, and all severities. |
| Doc tests | 1 | Adequate. The single doctest demonstrates construction. |
| Level-2 real-terminal | 0 | **Gap.** No dedicated `bt status-block` Level-2 test, though the underlying `Style::Border` and color primitives are Level-2 tested via `bt quote` and `bt block`. |

All tests pass:
- `cargo test -p biscuit-terminal --lib status_block` → 43 passed
- `cargo test -p biscuit-terminal --test status_block_parity` → 46 passed
- `cargo test -p biscuit-terminal-cli --test integration_test -- test_status_block` → 9 passed

---

## Spec Compliance

| Criterion | Status | Notes |
|-----------|--------|-------|
| `TreeRenderable` root with optional header/body/hint | ✅ | `to_render_node` |
| Layout seeded on root `NodeAttrs` | ✅ | Explicit `set_layout` |
| Stable classes on root and children | ✅ | `status-block`, `status-block--{severity}`, `status-block__header`, `status-block__body`, `status-block__hint` |
| Default border as thick left `Style::Border` | ✅ | `BorderWeight::Thick`, `left: true` |
| Arbitrary `border(String)` is terminal-only fallback | ✅ | `has_default_border()` gate |
| `TerminalRenderable` tree route for default border | ✅ | `render_via_tree` |
| `BrowserRenderable` delegates through tree | ✅ | Direct `render_browser_node` (matches evolved pattern; spec's `BrowserTreeComponent` sketch is outdated) |
| `MarkdownRenderable` delegates through `render_markdown_node` | ✅ | |
| `bt status-block` CLI with all required flags | ✅ | |
| `--md` / `--html` mutually exclusive | ✅ | `conflicts_with` |
| No CLI flag for arbitrary border prefix | ✅ | Explicitly absent; integration test verifies |
| Parity tests for semantic content, icons, border, color, layout | ✅ | `status_block_parity.rs` |
| Unit tests for tree structure, classes, style, layout | ✅ | In-source tests |
| Unit tests for Browser and Markdown | ✅ | |
| CLI integration tests for all targets and severities | ✅ | |
| Components table updated | ❌ | `renderable/docs/components.md` still shows ❌ for Browser/Markdown/Tree |

---

## Production Readiness

**Judgment: Production ready.**

The StatusBlock implementation is solid, well-tested, and follows the established migration recipe. The terminal tree path correctly reproduces the default thick left border, severity colors, portable icons, and layout margins. The bespoke compatibility fallback for arbitrary border prefixes is narrow and correctly isolated. The Browser and Markdown paths produce clean, structural output. All acceptance criteria are met except for the `components.md` table update, which is a documentation artifact rather than a runtime issue.

The one substantive gap is the absence of a dedicated Level-2 real-terminal test for `bt status-block`. However, the underlying `Style::Border` primitive is Level-2 proven via `bt quote`, and the generic color/fill primitives are Level-2 proven via `bt block`. The composition risk is low because StatusBlock's projection is a straightforward sequence of standard block nodes (`Paragraph`, `BlockQuote`, `Paragraph`) that the renderer already handles correctly. Adding a Level-2 test would strengthen confidence but is not a blocker for shipping.

**Recommended follow-ups before calling the feature fully closed:**
1. Update `renderable/docs/components.md` StatusBlock row.
2. Add `render_tree_node()` override for consistency with the Stage 2 recipe.
3. Add a Level-2 `bt status-block` test to verify composed border + color + layout in a real terminal emulator.
4. Add a unit test that asserts `border.color` value in the serialized tree JSON.

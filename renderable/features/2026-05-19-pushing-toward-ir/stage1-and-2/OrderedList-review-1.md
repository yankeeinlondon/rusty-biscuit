---
ready: true
---

# OrderedList — Implementation Review

> Review date: 2026-05-19
> Reviewer: Kimi Code CLI
> Component: `biscuit-terminal/lib/src/components/list.rs` (`OrderedList`)
> CLI: `biscuit-terminal/cli/src/commands/list.rs`
> Parity tests: `biscuit-terminal/lib/tests/ordered_list_parity.rs`
> Integration tests: `biscuit-terminal/cli/tests/integration_test.rs`

## Summary

`OrderedList` has been fully migrated to the canonical render-tree architecture. It implements `TreeRenderable`, `TerminalRenderable`, `BrowserRenderable`, and `MarkdownRenderable`. The default terminal render path routes through `render_via_tree`; the bespoke path is preserved as `render_bespoke()` for parity testing. The `bt list --ordered` CLI supports terminal, Markdown, MarkdownPlus, and HTML output.

The implementation follows the Stage 2 migration recipe faithfully: one private projection helper (`to_render_tree_node_with_terminal`), `render_via_tree` with `tracing::error!` fallback, dedicated parity tests, and cross-target CLI flags. The Prose-downcast pattern (via the shared `project_renderable_content` helper) ensures inline styling survives the terminal path.

---

## Findings

### 1. `layout_matrix` omits `OrderedList` despite documentation claiming coverage — **medium**

`biscuit-terminal/lib/tests/layout_matrix_support.rs` defines six component cases (`Section`, `UnorderedList`, `TwoColumn`, `Progress`, `Table`, `BlockQuote`). `OrderedList` is absent.

`renderable/docs/layout-and-style.md` §5 states:

> Seven components emit Layout (`Section`, `OrderedList`, `UnorderedList`, `Progress`, `TwoColumn`, `Table`, `BlockQuote` …). Their tree output is snapshot-tested in `layout_matrix` …

This is incorrect. `OrderedList` is not exercised by the layout matrix snapshot harness, so visual/layout regressions for ordered lists (e.g., prefix-width transitions, hanging-indent alignment at varying widths) are not captured by the automated snapshot gate.

**Recommended fix:** Add an `OrderedList` case to `component_cases()` in `layout_matrix_support.rs`, using a multi-item list that exercises at least one double-digit prefix so the snapshot captures prefix-width growth. Regenerate snapshots with `INSTA_UPDATE=always`.

### 2. `render_markdown` / `render_markdown_plus` silently swallow render errors — **medium**

```rust
fn render_markdown(&self) -> String {
    let node = self.render_tree();
    render_markdown_node(&node, &MarkdownRenderOptions::default())
        .map(|r| r.output)
        .unwrap_or_default()
}
```

A tree-validation or render failure returns an empty string with no diagnostic. The terminal path (`render_via_tree`) logs via `tracing::error!` before falling back to empty; the browser path returns a visible `[render-tree error: …]` fragment. The markdown path does neither.

Per the `Progress` polish lesson in `lessons-learned.md`:

> The right response to a tree-render failure is the same on both targets — log the error via `tracing::error!` … and return an empty output.

**Recommended fix:** Replace `unwrap_or_default()` with an explicit `match` that logs the error through `tracing::error!` (component = "OrderedList", dialect = "markdown" / "markdown_plus") before returning `String::new()`.

### 3. No explicit `NO_COLOR` integration test for `bt list --ordered` — **low**

The CLI uses `detect_terminal_honoring_force_color()`, which respects `NO_COLOR`, but there is no test that spawns `bt list --ordered` with `NO_COLOR=1` and asserts the absence of ANSI escapes. The generic `test_respects_no_color` only exercises the base `bt` command.

This is a user-observable accessibility requirement. Per `test-rigor.md`, user-observable behavior should have verification at the appropriate level. For a non-interactive structural component, Level-1 integration tests are sufficient; the gap is that the specific subcommand is not exercised.

**Recommended fix:** Add a CLI integration test:

```rust
#[test]
fn test_list_ordered_respects_no_color() {
    cargo_bin_cmd!("bt")
        .env("NO_COLOR", "1")
        .args(["list", "--ordered", "First", "Second"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not());
}
```

### 4. `KNOWN_DRIFT` ledger and `render_comparison.rs` do not mention `OrderedList` — **low**

`render_comparison.rs` retires drift entries for flipped components with explanatory comments. `OrderedList` is not mentioned, but neither is it exercised by the layout matrix harness, so there is no drift to record or retire. This is consistent but worth noting: the absence of `OrderedList` from the harness means the automated comparison gate does not cover it.

Once Finding 1 is addressed (adding `OrderedList` to `layout_matrix_support.rs`), a drift entry may or may not appear. If the snapshot shows no drift, no ledger entry is needed; if drift surfaces, it should be documented.

### 5. `render_html_with_layout` comment refers to `list.rs` instead of `ol`-specific reasoning — **nit**

The comment in `cli/src/commands/list.rs` at `render_html_with_layout` is copy-paste accurate but could be more explicit that the function applies to *both* ordered and unordered lists. This is not a bug — the code is correct — but a reader might wonder whether the wrapper-only logic is list-kind-aware.

---

## What is well done

- **Single projection helper.** `to_render_tree_node_with_terminal` is the sole source of truth for `TreeRenderable::render_tree`, `TerminalRenderable::render_tree_node`, and `render_via_tree`. The parity test `tree_renderable_and_compat_hook_share_one_projection` serializes both entry points and asserts equality.
- **Prose downcast survives inline styling.** `project_list_items` downcasts `Prose` components and wraps `to_render_nodes()` in a `Paragraph`, so `<b>Bold</b>` items emit `\x1b[1m…\x1b[22m` on the terminal target. The regression test `prose_inline_styling_survives_terminal_render` pins the SGR byte.
- **HTML double-application guard.** `test_list_ordered_html_with_layout_emits_margin_on_ol_only` asserts `margin-left` appears exactly once and no `<div style=…>` wrapper is emitted for tree-expressible properties. This matches the OrderedList-spec requirement exactly.
- **Comprehensive parity coverage.** `ordered_list_parity.rs` covers empty lists, single/multi items, double/triple-digit prefixes, word wrap, nested ordered/unordered lists, mixed inline/block children, custom `indent_children`, margins, alignment, and markdown/browser output.
- **CLI cross-target uniformity.** `--ordered` / `-o`, `--md`, `--md-plus`, `--html`, and `--example` are all wired and tested at the integration level. Mutual exclusion of target flags is enforced by clap and tested.
- **Error fallback policy is almost uniform.** Terminal path logs + empties; browser path emits a diagnostic fragment. Only the markdown path is inconsistent (Finding 2).

---

## Production Readiness

**Judgment: `true` — the component is production ready.**

`OrderedList` meets the functional and structural bar for shipping:

1. **All four target traits are implemented** and route through the canonical render tree.
2. **The default terminal path is flipped** to the tree renderer; the bespoke path is retained only for parity testing.
3. **Parity tests** demonstrate semantic equivalence between bespoke and tree output across the spec's full variant matrix.
4. **Integration tests** verify the CLI surface for all three cross-target flags.
5. **The Prose-downcast pattern** ensures inline styling survives the terminal path — a regression that was caught and fixed during migration.

The findings above are **test-harness and observability gaps**, not functional defects:

- Finding 1 (missing from `layout_matrix`) is a snapshot-coverage gap. The component is already well-tested at Level-1; the layout matrix would add visual regression insurance but does not change the correctness of the implementation.
- Finding 2 (silent markdown errors) is an observability inconsistency. In practice, `OrderedList` projects to a structurally trivial tree that validates cleanly; the failure path is effectively unreachable for normal inputs. Still, the logging should be added for uniformity.
- Finding 3 (missing `NO_COLOR` test) is a test gap in a shared code path (`detect_terminal_honoring_force_color`) that is already tested for other subcommands.

None of these gaps represent user-visible broken behavior. The component renders correctly to Terminal, Markdown, MarkdownPlus, and Browser; the CLI is accessible and consistent; and the parity discipline proves the tree path preserves the component's semantics. `OrderedList` is ready for production use.

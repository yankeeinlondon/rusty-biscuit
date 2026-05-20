---
ready: false
---

# Compose Component Review

> Review of the `Compose` component's IR migration against the specification at
> `renderable/features/2026-05-19-pushing-toward-ir/components/Compose-spec.md`.

## Executive Summary

The core implementation of `Compose`'s tree-backed rendering is **functionally correct and well-tested at Level 1**. `TreeRenderable`, `TerminalRenderable`, `MarkdownRenderable`, and `BrowserRenderable` are all implemented and route through the canonical render tree. `RT-COMPOSE-001` (sequence-join `None`) is present and exercised. The `bt compose` CLI is registered and tested.

However, **the acceptance criteria in the spec are not fully satisfied**. The component lacks a dedicated parity-test file (the established pattern for every other flipped component), has no `KNOWN_DRIFT` ledger (an explicit acceptance criterion), carries stale end-user documentation, and has inconsistent cross-target error-handling policy. These are process and contract gaps rather than functional bugs, but they block a "production ready" judgment.

---

## Findings

### 1. Missing dedicated parity-test file [`HIGH`]

**Gap:** Every other flipped component (`BlockQuote`, `OrderedList`, `UnorderedList`, `Section`, `Progress`) has a standalone parity test under `biscuit-terminal/lib/tests/` (e.g. `section_parity.rs`, `ordered_list_parity.rs`). `Compose` does not have a `compose_parity.rs`.

**Why this matters:** The spec's acceptance criteria state "Parity tests cover all variants listed above." While `compose.rs` contains 116 unit tests that touch many of those variants (terminal concatenation, Markdown no-separator, browser DOM order, layout margins, nested Compose), they are embedded in the component's own test module rather than in the canonical parity harness. The parity-file pattern provides:

- A `render_bespoke` fallback for byte-level or token-level historical comparison.
- A width-matrix sweep (`PARITY_WIDTHS`).
- Explicit structural assertions (e.g. "nested block component must appear as its own kind, not flat text").
- Cross-target coverage (Terminal, Markdown, Browser) in one discoverable location.

**Evidence:** `grep -r 'compose_parity' biscuit-terminal/lib/tests/` returns nothing. The `render_tree_component_parity.rs` file only covers `BlockQuote`.

**Recommended fix:** Create `biscuit-terminal/lib/tests/compose_parity.rs` following the pattern in `section_parity.rs`. It should include:

1. Structural snapshots (`render_tree_node_produces_root_with_sequence_join_none`, nested Compose hoisting).
2. Validation gate (`projected_tree_validates_with_no_errors`).
3. Semantic parity between a retained `render_bespoke` and the tree path.
4. Width matrix (`PARITY_WIDTHS`).
5. Markdown / Browser / MarkdownPlus cross-target assertions.
6. A `render_bespoke` compatibility method on `Compose` (or a standalone helper) so the parity gate has a historical baseline to compare against.

---

### 2. No `KNOWN_DRIFT` ledger [`HIGH`]

**Gap:** The spec explicitly requires: "`KNOWN_DRIFT` ledger documents accepted divergences." There is no such ledger for Compose.

**Why this matters:** The spec lists four accepted divergences:

- **Prose styling loss**: Parts that are `Prose` components lose some styling in the generic terminal-content projection until Prose has full tree projection coverage.
- **Adapter block-level reporting**: `TreeComponent<Compose>` may report `true`; `Compose` itself should continue to report `false`.
- **Heading escape ordering**: Bespoke headings and tree headings may emit different SGR sequences.
- **HTML wrapper**: `render_browser_node` on a root sequence emits a wrapper `<div>`, while `render_browser_document` renders root children as page body fragments.

These are currently scattered in code comments or lessons-learned, but there is no single, discoverable ledger that a future maintainer can read. Other components record drift in `render_comparison.rs` or in dedicated parity files.

**Recommended fix:** Add a `KNOWN_DRIFT` block either in a new `compose_parity.rs` file or as a doc comment at the top of `compose.rs`. Each entry should name the divergence, explain why it is accepted, and reference the test that gates it.

---

### 3. Inconsistent cross-target error-handling policy [`MEDIUM`]

**Gap:** The three render targets handle `render_*_node` failures differently, violating the uniform policy established during the `Progress` migration.

| Target   | Current behavior                              | Expected policy (per lessons-learned) |
|----------|-----------------------------------------------|---------------------------------------|
| Terminal | `tracing::error!(...) + String::new()`       | ✅ Log + empty output                 |
| Markdown | `.unwrap_or_default()` (silent drop)          | ❌ Should log + empty output          |
| Browser  | In-band `[render-tree error: …]` text fragment | ❌ Should log + empty fragment        |

**Evidence:**

```rust
// Terminal — correct
Err(error) => {
    tracing::error!(...);
    String::new()
}

// Markdown — silent
render_markdown_node(&node, &opts)
    .map(|r| r.output)
    .unwrap_or_default()

// Browser — in-band sentinel
Err(error) => BrowserFragment::new()
    .define_as_text_fragment(format!("[render-tree error: {error}]"))
    .finalize()
```

**Recommended fix:** Unify all three paths to `tracing::error!(component = "Compose", target = "...", error = %error)` plus an empty fallback. For `BrowserFragment`, an empty fragment is `BrowserFragment::new().finalize()`. For Markdown, replace `unwrap_or_default()` with an explicit `match` that logs.

---

### 4. Stale end-user documentation [`MEDIUM`]

**Gap:** `biscuit-terminal/docs/components/compose.md` states:

> "Not directly exposed as a CLI command. Compose is a programmatic building block used internally to assemble complex outputs from other components."

This is no longer true. `bt compose` is registered, implemented, and tested.

**Recommended fix:** Update `docs/components/compose.md` to document the `bt compose` CLI, its flags, and its part-ordering semantics. Mirror the style of `docs/components/prose.md` or `docs/components/section.md`.

---

### 5. No Level 2 (real-terminal) verification [`MEDIUM`]

**Gap:** Per `prompts/snippets/test-rigor.md`, user-observable terminal behavior must have verification at the appropriate level. Compose has:

- **Level 1:** 116 lib unit tests + 15 CLI integration tests (PTY-less process spawn).
- **Level 2:** None. No `cli/tests/level2_compose*.rs`.
- **Level 3:** N/A (no keyboard interaction).

The terminal path exercises sequence-join semantics, inline SGR lowering (Prose bold), layout margins, and alignment. These are all user-visible rendering behaviors that benefit from Level 2 capture (e.g. verifying that bold SGR actually renders as bold glyphs in WezTerm/Kitty, or that connector geometry survives a real terminal's width handling).

**Recommended fix:** Add `cli/tests/level2_compose.rs` following the pattern of `level2_render_tree_style.rs` or `level2_prose_styling.rs`. Minimum coverage:

- `bt compose --prose "<b>bold</b>"` captured via `wezterm cli get-text` or `tmux capture-pane`, asserting glyph presence.
- `bt compose` with layout flags (`--margin-left`) asserting real-terminal indent width.

---

### 6. `render_bespoke` not preserved [`LOW`]

**Gap:** Unlike `Section`, `OrderedList`, `UnorderedList`, and `Progress`, `Compose` does not retain a `render_bespoke` method. The spec acceptance criteria says `TerminalRenderable` delegates to the tree path "after parity passes," but the established migration pattern keeps the bespoke renderer as `#[doc(hidden)] pub fn render_bespoke` for historical comparison.

**Mitigation:** Compose's bespoke renderer was trivial (concatenate `part.render()` strings), so the loss is small. However, retaining it would allow a true parity gate in `compose_parity.rs` and would be consistent with the monorepo's migration discipline.

**Recommended fix:** Add a `#[doc(hidden)] pub fn render_bespoke(&self, term: &Terminal) -> String` that performs the old concatenation loop, gated behind `#[cfg(test)]` or `#[doc(hidden)]`.

---

### 7. CLI `apply_layout_args` ignores `margin_top` / `margin_bottom` on the component [`LOW` — documented limitation]

**Observation:** `bt compose/cli/src/commands/compose.rs::apply_layout_args` only forwards `margin_left`, `margin_right`, and `alignment` to the `Compose` layout. `margin_top` and `margin_bottom` are handled by `emit_vertical_margins` in the CLI runner.

This is **intentional and consistent** with other commands (`bt section`, `bt quote`, `bt list`), but it means that programmatic users calling `compose.render(&term)` with a non-default `layout.margin.top` will see the tree renderer honor it, while CLI users get the margin from `emit_vertical_margins`. The two paths are not identical (the tree renderer applies margins within the render pipeline; `emit_vertical_margins` prints blank lines around `println!`).

This is a documented shared limitation (`LayoutArgs` does not expose `--max-width` or `--word-wrap` either). No action required unless the team wants to unify vertical-margin application.

---

## What is implemented well

- **Core tree projection is correct.** `SequenceJoin::None` is seeded, nested `Root` nodes are flattened, `Prose` downcasts survive inline styling, and bespoke-only children fall back via the threaded terminal.
- **Unit-test breadth is strong.** 116 tests cover construction, `From` impls, every `add_*` method, layout parity, terminal/Markdown/Browser rendering, Unicode, many-part stress, and strictness gates.
- **CLI integration tests are thorough.** 15 tests cover argument parsing, mutual exclusion, all three cross-target flags (`--md`, `--md-plus`, `--html`), `--example`, and Prose SGR preservation.
- **`is_block_level()` discipline is respected.** The component correctly stays `false`, preserving the public contract.
- **Sequence-join hot path is performant.** `render_sequence` pushes `&str` directly instead of cloning per `Text` child.

---

## Production Readiness

**Judgment: NOT production ready.**

**Why:** The functional implementation is sound, but the spec's acceptance criteria act as a checklist for completion, and two explicit items are missing:

1. **Parity tests** — There is no `compose_parity.rs` following the established pattern.
2. **`KNOWN_DRIFT` ledger** — Accepted divergences are not documented in a discoverable ledger.

Additionally, the cross-target error-handling policy is inconsistent with the project's evolved standards, end-user documentation is stale, and there is no Level 2 real-terminal verification. These are not theoretical concerns: the absence of a parity file means a future refactor of `render_sequence` or `project_part` could introduce a regression in bespoke-vs-tree fidelity with no dedicated gate to catch it.

**Path to ready:**

1. Create `biscuit-terminal/lib/tests/compose_parity.rs` with structural, semantic, width-matrix, and cross-target parity coverage.
2. Add `#[doc(hidden)] pub fn render_bespoke` to `Compose` for historical comparison.
3. Write the `KNOWN_DRIFT` ledger in the parity file.
4. Unify error handling across Terminal/Markdown/Browser to `tracing::error!` + empty fallback.
5. Update `docs/components/compose.md` to reflect the `bt compose` CLI.
6. Add `cli/tests/level2_compose.rs` with at least one real-terminal capture.

---
ready: false
---

# Todo Component — Implementation Review

> Review date: 2026-05-20
> Reviewer: Kimi Code CLI
> Component: `biscuit-terminal/lib/src/components/todo.rs`
> CLI: `biscuit-terminal/cli/src/commands/todo.rs`
> Tests: `biscuit-terminal/lib/tests/todo_parity.rs`
> Spec: `renderable/features/2026-05-19-pushing-toward-ir/components/Todo-spec.md`

---

## Summary

The `Todo` component has a **structurally sound implementation** that correctly implements all four required traits (`TreeRenderable`, `TerminalRenderable`, `BrowserRenderable`, `MarkdownRenderable`), wires the terminal default render path through the canonical tree, provides a working `bt todo` CLI with all specified flags, and updates the components table. However, **critical test-coverage gaps remain** that prevent calling this production-ready under the repo's own parity-gate discipline. The component is effectively "code-complete but verification-incomplete."

---

## Findings

### 1. Systematic parity gate is missing — **severity: critical**

`Todo` is **not included** in the `render_comparison.rs` matrix (`biscuit-terminal/lib/tests/render_comparison.rs`). This is the workspace's central `KNOWN_DRIFT` ledger and the single strongest guard against bespoke-vs-tree drift. Every other flipped component (`BlockQuote`, `OrderedList`, `UnorderedList`, `Progress`, `Table`, `Section`, `Compose`, `TwoColumn`, `TextBlock`) participates in this gate.

**Impact:** Without `Todo` in the matrix, there is no automated detection if a future change to the terminal tree renderer's `ListItem` handler, `task_state_marker`, or `Layout` application breaks parity with `render_bespoke`. The ad-hoc `todo_parity.rs` tests cover structural shape and coarse content preservation, but they do not exercise the six-facet comparison (exact bytes, visible text, indentation, blank lines, width, styling) that `render_comparison.rs` enforces.

**Required fix:** Add `Todo` scenarios to `layout_matrix_support.rs` (or the equivalent component list used by `render_comparison.rs`), regenerate snapshots, and populate any resulting `KNOWN_DRIFT` entries.

---

### 2. Spec parity variants are incomplete — **severity: high**

The spec lists 15 critical parity variants (§Terminal IR Implementation → Parity Test Strategy). Coverage in `todo_parity.rs`:

| Variant | Status | Gap |
|---------|--------|-----|
| Open / Completed / InProgress / Blocked / Cancelled states | Partial | Only tree path is tested; `render_bespoke` vs `render` byte-level comparison is absent except for a weak "contains Description" check in NoColor. |
| Plain description | ✅ | Covered by `every_state_preserves_description_text_through_tree`. |
| **Prose description (`use_prose = true`)** | **Missing** | No test renders a `Todo::from_prose(...)` through either path. The spec explicitly calls this a critical variant. |
| **Nerd Font terminal — InProgress, Blocked, Cancelled** | **Missing** | Only Open and Completed are tested (`nerd_font_terminal_uses_nerd_glyph_open` / `_completed`). |
| **No-color terminal — all states** | **Partial** | Individual unit tests exist for bespoke `to_terminal`, but the tree path is only spot-checked (Open, Completed, Cancelled). No systematic bespoke-vs-tree marker comparison. |
| **TrueColor terminal — all states** | **Partial** | Only `test_color_completed_todo_has_ansi` exists; other states are not asserted to emit color-specific SGR. |
| Left margin applied | ✅ | `layout_left_margin_applies_through_tree`. |
| **Right margin applied** | **Missing** | No test asserts right margin narrows available width. |
| **Center alignment** | **Missing** | No test asserts centered output. |
| Empty description | Partial | Tree path only (`empty_description_renders_only_marker`). Bespoke path not compared. |
| Description with special characters | Partial | Tree path only (`special_characters_preserved_through_tree`). Bespoke path not compared. |

**Required fix:** Expand `todo_parity.rs` with the missing variants, especially `use_prose = true` and alignment/margin coverage.

---

### 3. CLI integration test omits `todo` — **severity: high**

`biscuit-terminal/cli/tests/integration_test.rs::test_every_subcommand_help_exposes_example_flag` does not include `"todo"` in its subcommand list (lines 48–73). This means the `--example` flag exposure is not verified by the CLI test suite.

**Required fix:** Add `"todo"` to the subcommand array.

---

### 4. No Level-2 (real-terminal) tests — **severity: medium**

There are no tests in `biscuit-terminal/cli/tests/level2_*.rs` that exercise `bt todo` inside a real terminal emulator (WezTerm, Kitty, tmux). For a component whose primary value proposition is terminal-adaptive checkbox glyphs (Nerd Font vs colored fallback vs no-color ASCII), Level-2 verification is the only way to confirm that glyph width, color SGR, and strikethrough actually render correctly through a real encoder/decoder.

**Required fix:** Add a Level-2 test that runs `bt todo --example` (or explicit state variants) and captures pane text, asserting marker presence.

---

### 5. `KNOWN_DRIFT` is fragmented and not in the central ledger — **severity: medium**

The spec requires a `KNOWN_DRIFT` ledger documenting accepted divergences. `todo_parity.rs` contains an inline comment (in `no_color_no_nerd_non_cancelled_emits_no_ansi_escapes`) noting that Cancelled's tree path preserves strikethrough SGR even in `ColorDepth::None` while the bespoke path stripped it. This divergence is:
- Not present in `render_comparison.rs`'s `KNOWN_DRIFT` (because Todo isn't in the matrix at all).
- Not documented as a formal `KNOWN_DRIFT` block inside `todo_parity.rs`.
- Not tested as an explicit drift — the test simply skips Cancelled when asserting "no ANSI."

Additionally, the spec lists **Prose styling loss** as an accepted drift, but there is no test or comment documenting it for Todo.

**Required fix:** Once Todo is added to `render_comparison.rs`, record the Cancelled NoColor strikethrough drift in the central ledger. Add a dedicated `KNOWN_DRIFT` comment block in `todo_parity.rs` mirroring the BlockQuote/OrderedList precedent.

---

### 6. Bespoke parity assertion is too weak — **severity: medium**

`bespoke_and_tree_share_description_in_no_color_terminal` asserts only that `bespoke.contains("Description") && tree.contains("Description")`. It does **not** assert that the checkbox markers match (e.g., both emit `[x]` for Completed, both emit `[-]` for Cancelled). A regression that caused the tree path to emit `[ ]` for every state would pass this test.

**Required fix:** Strengthen the assertion to compare stripped output directly, or at least assert marker presence per state.

---

### 7. `use_prose` description flattens inline styling — **severity: low (documented gap)**

`Todo::description_text()` renders `Prose` optimistically and strips ANSI to recover plain text. This matches the BlockQuote precedent and is the documented lossy projection. However, unlike BlockQuote, Todo has **no test** that verifies Prose content survives (e.g., `<b>bold</b> description` → tree output contains "bold description"). The risk is low because the code path is identical to BlockQuote's, but an untested path is an unguaranteed path.

**Required fix:** Add a tree-render test with `Todo::from_prose("<b>bold</b> task")` asserting the description text is present after ANSI stripping.

---

### 8. Browser `Style` lowering is unimplemented at the renderer level — **severity: low (workspace gap)**

Todo seeds a `Style` with `dim + strikethrough` on the `ListItem` for Cancelled. The Browser renderer does not yet lower `Style` to CSS (documented in `layout-and-style.md` §6: "Browser `Style` lowering is designed but not yet wired"). This is a workspace-wide gap, not a Todo-specific defect, but it means the Browser target does not currently realize the full Cancelled visual state.

**Note:** This does **not** block Todo production readiness by itself — it is correctly classified as a renderer gap, not a component gap.

---

## Positive Observations

- **Single projection helper:** `Todo::to_render_node()` is the sole source of truth for both `TreeRenderable::render_tree` and `TerminalRenderable::render_tree_node`, preventing drift between the canonical and compatibility hooks.
- **Error handling:** All infallible trait fallbacks (`render_via_tree`, `render_markdown`, `render_html_fragment`) log via `tracing::error!` and return empty output, following the BlockQuote/Progress precedent.
- **TaskHints validation:** The renderable tree validator correctly rejects `task_hints` on non-`ListItem` nodes, and unit tests in `validate.rs` cover this.
- **CLI ergonomics:** The `bt todo` command correctly handles `--example`, `--prose`, layout args, and cross-target flags. The `--example` defaulting to `Completed` state is a thoughtful UX touch.
- **Component table updated:** `renderable/docs/components.md` correctly lists Todo as `both avail, tree renders` with `bt CLI = tree`.

---

## Production Readiness

**Judgment: NOT production ready.**

The implementation code is correct, complete, and follows established patterns. The blocker is **verification coverage**. Under this repo's own rules, a flipped component must:

1. **Pass the systematic parity gate** (`render_comparison.rs`) — Todo does not participate.
2. **Expose every user-facing CLI flag through integration tests** — `test_every_subcommand_help_exposes_example_flag` omits `todo`.
3. **Cover all spec-critical variants** — Several parity matrix cells (Prose, right margin, center alignment, full Nerd Font matrix, full TrueColor matrix) are untested.

Until these gaps are closed, a future refactor of the terminal list renderer or the `task_state_marker` helper could silently break Todo's visual output without any CI failure. The code is ready; the test harness is not.

**Checklist to reach ready:**

- [ ] Add `Todo` to `render_comparison.rs` matrix and populate `KNOWN_DRIFT`.
- [ ] Add `"todo"` to `test_every_subcommand_help_exposes_example_flag`.
- [ ] Add `Todo::from_prose` tree and parity tests.
- [ ] Add right-margin and center-alignment tree tests.
- [ ] Add Nerd Font tree tests for InProgress, Blocked, and Cancelled.
- [ ] Add TrueColor SGR assertions for all five states.
- [ ] Strengthen `bespoke_and_tree_share_description_in_no_color_terminal` to compare markers.
- [ ] (Optional but recommended) Add Level-2 `bt todo` real-terminal test.

---
phases: 2
created: 2026-05-09
start_phase: 1
---

# Execution Plan: Good Errors

This plan outlines the implementation of rich, source-aware error diagnostics in darkmatter. It follows the [Functional Specification](./spec.md).

## Phase 1: Reference Implementation (Foundation)

This phase establishes the new API in `biscuit-terminal` and implements a complete reference migration for `PageBlockError::UnterminatedBlock`.

### Task 1.1: `biscuit-terminal` API Changes
- [ ] Modify `StatusBlock` struct in `biscuit-terminal/lib/src/components/status_block.rs`:
    - Change `body` field from `Option<RenderableContent>` to `Vec<Prose>`.
- [ ] Update `StatusBlock` implementation:
    - [ ] Implement `fn body(self, body: impl Into<Vec<Prose>>) -> Self`.
    - [ ] Implement `fn body_line(self, line: impl Into<Prose>) -> Self`.
    - [ ] Add `From<Prose> for Vec<Prose>` conversion.
- [ ] Update `StatusBlock::render`:
    - [ ] Iterate over `self.body`, rendering each `Prose` item.
    - [ ] Join rendered segments with blank lines (wrapped with border glyphs) to maintain a continuous block.
- [ ] **Validation:** `cargo test -p biscuit-terminal` (expect compilation errors in other packages).

### Task 1.2: `SourceContext` and Rendering Helpers
- [ ] Create `biscuit-terminal/lib/src/errors/source_context.rs`.
- [ ] Define `SourceContext` struct with:
    - `absolute: PathBuf`
    - `display: PathBuf`
    - `content: Arc<str>`
    - `frontmatter: Option<std::ops::Range<usize>>`
- [ ] Implement rendering methods on `SourceContext`:
    - [ ] `fn linked_path_prose(&self) -> Prose`
    - [ ] `fn frontmatter_prose(&self) -> Option<Prose>`
    - [ ] `fn excerpt_prose(&self, line: usize, context: usize, lang: &str) -> Prose` (implementing the gutter logic from spec).
- [ ] **Validation:** Unit tests for `SourceContext` helpers in `biscuit-terminal`.

### Task 1.3: Prose Fenced Code Block Grammar
- [ ] Update the `Prose` parser/renderer in `biscuit-terminal` to support minimal fenced code blocks:
    - [ ] Match ` ```LANG\n ... ``` ` syntax.
    - [ ] Render body with line preservation, 2-space indent, and dim foreground color.
- [ ] **Validation:** `Prose` unit tests verifying fenced block rendering.

### Task 1.4: Reference Migration - `PageBlockError::UnterminatedBlock`
- [ ] Update `PageBlockError` in `darkmatter/lib/src/markdown/compose/page_blocks/types.rs`:
    - [ ] Add `source: SourceContext` to `UnterminatedBlock`.
    - [ ] Rename `line` to `opening_line`.
    - [ ] Drop `file_ends_at_line`.
- [ ] Thread `SourceContext` through the page block parser (`darkmatter/lib/src/markdown/compose/page_blocks/parser.rs`).
- [ ] Implement `BlockError::status_block` for `UnterminatedBlock` in `darkmatter/lib/src/markdown/compose/page_blocks/render.rs` (or equivalent) following the spec's structural requirements (Linked Header -> Frontmatter -> Excerpt -> Hint).
- [ ] **Validation:** Manually verify the rendered output of `UnterminatedBlock` to ensure no bare tags are visible.

### Task 1.5: Workspace-wide Call Site Migration
- [ ] Identify all remaining `StatusBlock::body` call sites in the workspace (using `grep_search`).
- [ ] Mechanically update call sites to use `body_line(...)` or `body(vec![...])` to restore compilation.
- [ ] **Validation:** `cargo check --workspace` passes.

### Task 1.6: Documentation & Snapshot Testing
- [ ] Create `darkmatter/docs/errors/README.md` as described in the spec.
- [ ] Add `insta` as a dev-dependency to `darkmatter` if not present.
- [ ] Implement a snapshot test for `PageBlockError::UnterminatedBlock`.
- [ ] **Validation:** `cargo test -p darkmatter` passes with a checked-in snapshot.

---
**Checkpoint Phase 1:** `biscuit-terminal` foundation is solid, and the first darkmatter error is rich and source-aware.

## Phase 2: Full Sweep

This phase migrates the remaining error variants in `darkmatter` to the new pattern.

### Task 2.1: Migrate Transclusion Errors (Parallelizable)
- [ ] Update variants in `darkmatter/lib/src/markdown/compose/transclusion/types.rs`.
- [ ] Thread `SourceContext` through `transclusion/engine.rs`.
- [ ] Add snapshot tests for all variants.

### Task 2.2: Migrate Reference Errors (Parallelizable)
- [ ] Update variants in `darkmatter/lib/src/markdown/reference/errors.rs`.
- [ ] Add snapshot tests for all variants.

### Task 2.3: Migrate Render Errors (Parallelizable)
- [ ] Update variants in:
    - [ ] `darkmatter/lib/src/render/stylesheet.rs`
    - [ ] `darkmatter/lib/src/render/image_ref.rs`
    - [ ] `darkmatter/lib/src/render/link.rs`
- [ ] Add snapshot tests for all variants.

### Task 2.4: Complete PageBlockError Migration
- [ ] Migrate `ParseDirective`, `UnmatchedEnd`, and `Condition` variants in `PageBlockError`.
- [ ] Add snapshot tests for each.

### Task 2.5: Final Polish & Maintenance
- [ ] Create/Update `.claude/skills/darkmatter/errors.md` with instructions for future error implementation.
- [ ] Update `darkmatter/README.md` and `biscuit-terminal/README.md` to reflect the changes.
- [ ] Ensure all `StatusBlock` usages in the workspace are clean and leverage `Prose` correctly.

---
**Checkpoint Phase 2:** Every darkmatter error that has a file origin now provides a linked path, frontmatter context (if applicable), and a source excerpt.

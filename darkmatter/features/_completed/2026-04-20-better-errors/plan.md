---
phases: 4
created: 2026-04-20
start_phase: 1
source_files_during_phase_1:
  - biscuit-terminal/lib/src/errors/mod.rs
  - biscuit-terminal/lib/src/errors/block_error.rs
  - biscuit-terminal/lib/src/lib.rs
  - biscuit-terminal/lib/src/prelude.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2:
  - darkmatter/cli/src/main.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/types.rs
  - darkmatter/lib/src/markdown/compose/context/merge.rs
  - darkmatter/lib/src/markdown/reference/errors.rs
  - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/mermaid/theme.rs
  - darkmatter/cli/src/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/normalize/types.rs
  - darkmatter/lib/src/render/stylesheet.rs
  - darkmatter/lib/src/render/link.rs
  - darkmatter/lib/src/render/image_ref.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/tests/error_snapshots/main.rs
  - darkmatter/lib/tests/error_snapshots/helpers.rs
  - darkmatter/lib/tests/error_snapshots/ctx_merge.rs
  - darkmatter/lib/tests/error_snapshots/deferred_set.rs
  - darkmatter/lib/tests/error_snapshots/editor.rs
  - darkmatter/lib/tests/error_snapshots/file_tree.rs
  - darkmatter/lib/tests/error_snapshots/image_ref.rs
  - darkmatter/lib/tests/error_snapshots/link.rs
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
  - darkmatter/lib/tests/error_snapshots/mermaid_theme.rs
  - darkmatter/lib/tests/error_snapshots/normalization.rs
  - darkmatter/lib/tests/error_snapshots/page_block.rs
  - darkmatter/lib/tests/error_snapshots/reference.rs
  - darkmatter/lib/tests/error_snapshots/shell_expansion.rs
  - darkmatter/lib/tests/error_snapshots/stylesheet.rs
  - darkmatter/lib/tests/error_snapshots/toc_linking.rs
  - darkmatter/lib/tests/error_snapshots/transclusion.rs
  - darkmatter/lib/tests/error_snapshots/condition.rs
docs_updated_during_phase_4:
  - biscuit-terminal/README.md
docs_created_during_phase_4:
  - darkmatter/docs/error-rendering.md
skills_files_updated_during_phase4: []
packages:
  - biscuit-terminal
  - darkmatter
  - darkmatter-cli
---

# Better Errors — Execution Plan

This plan outlines the steps to implement the `BlockError` trait and adopt it across the `darkmatter` library for high-quality, terminal-aware error reporting.

## Phase 1: Foundation (biscuit-terminal)

**Goal:** Establish the core trait and rendering helpers in the shared `biscuit-terminal` library.

1.  **Step 1: Implement `BlockError` Trait**
    - Create `biscuit-terminal/lib/src/errors/block_error.rs`.
    - Define `BlockError` trait with `status_block`, `severity`, `report_block_error`, and `report_block_error_optimistic` methods.
    - [x] *Parallelizable:* No.
2.  **Step 2: Implement Rendering Helpers**
    - Implement `ErrorHeader` struct for standardized `<b>Name:</b> <b>Title</b>` formatting.
    - Implement `StatusBlockExt` trait to provide `.error_header(ErrorHeader)` on `StatusBlock`.
    - [x] *Parallelizable:* Yes (with Step 1).
3.  **Step 3: Implement Cause Chain Rendering**
    - Implement `render_with_causes` helper to handle nested `BlockError` instances.
    - Implement internal `as_block_error` helper for dynamic trait object discovery.
    - [x] *Parallelizable:* No (depends on Step 1).
4.  **Step 4: Validation & Exports**
    - Add unit tests in `biscuit-terminal` covering optimistic rendering and severity defaults.
    - Export `BlockError` and helpers in `biscuit-terminal/lib/src/prelude.rs` and `lib.rs`.
    - [x] *Validation Checkpoint:* `cargo test -p biscuit-terminal` passes.

## Phase 2: CLI Plumbing (darkmatter/cli)

**Goal:** Wire the `md` CLI to prefer `BlockError` rendering.

1.  **Step 1: Update Top-level Error Handler**
    - Modify `darkmatter/cli/src/main.rs`.
    - Detect if the caught error implements `BlockError` using `as_block_error`.
    - Render via `report_block_error` if TTY, or `report_block_error_optimistic` if piped.
    - [x] *Parallelizable:* No (depends on Phase 1).
2.  **Step 2: Validation**
    - Run `md` with a triggered error (e.g., missing file) and verify it still prints the old `Display` message (since no `BlockError` impls exist yet in `darkmatter`).
    - [x] *Validation Checkpoint:* `md` CLI continues to function correctly with existing errors.

## Phase 3: High-Value Variants (darkmatter/lib)

**Goal:** Implement `BlockError` for the most impactful error types.

1.  **Step 1: MarkdownError Wrapper Implementation**
    - Implement `BlockError` for `MarkdownError` in `darkmatter/lib/src/markdown/types.rs`.
    - Use the delegation strategy for sub-errors and implement initial leaf blocks for `FileLoad` and `UrlFetch`.
    - [x] *Parallelizable:* No (depends on Phase 1).
2.  **Step 2: Priority Group 1 (Transclusion & Shell)**
    - Implement `BlockError` for `TransclusionError` and `ShellExpansionError`.
    - Add enrichments: `CycleDetected` (line numbers), `ExecutionFailed` (stdout/stderr).
    - [x] *Parallelizable:* Yes.
3.  **Step 3: Priority Group 2 (Blocks & Conditions)**
    - Implement `BlockError` for `PageBlockError`, `ConditionError`, and `TocLinkingError`.
    - Add enrichments: `UnterminatedBlock` (echo directive), `Parse` (caret support).
    - [x] *Parallelizable:* Yes.
4.  **Step 4: Priority Group 3 (Reference & Editor)**
    - Implement `BlockError` for `ReferenceError`, `EditorError`, `FileTreeError`, and `MermaidThemeError`.
    - Add enrichments: `NoEditorFound` (discovery list), `PathNotFound` (absolute path).
    - [x] *Parallelizable:* Yes.
5.  **Step 5: Validation Checkpoint**
    - Manually trigger one error from each group using the `md` CLI.
    - Verify visual alignment and color application in the terminal.
    - [x] *Validation Checkpoint:* `just test` and `just lint` both pass for the `darkmatter` package area; new unit tests cover every variant enrichment.

## Phase 4: Coverage & Polish

**Goal:** Complete the remaining implementations and finalize documentation.

1.  **Step 1: Remaining Enums Coverage**
    - Implement `BlockError` for `DeferredSetError`, `NormalizationError`, `StylesheetError`, `LinkError`, and `ImageRefError`.
    - Add remaining enrichments as defined in the Technical Design (§2.3).
    - [x] *Parallelizable:* Yes.
2.  **Step 2: Snapshot Testing Suite**
    - Create `darkmatter/lib/tests/error_snapshots/`.
    - Implement a test harness to generate and verify snapshots for all 100+ error variants.
    - [x] *Validation Checkpoint:* `cargo test -p darkmatter-lib` includes snapshot coverage.
3.  **Step 3: Documentation & Skills Update**
    - Update `biscuit-terminal/README.md` and `darkmatter/docs/error-rendering.md`.
    - Update `biscuit-terminal` and `darkmatter` skill definitions.
    - [x] *Parallelizable:* Yes.

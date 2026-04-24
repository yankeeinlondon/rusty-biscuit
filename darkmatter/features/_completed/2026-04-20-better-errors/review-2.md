---
reviewer: gemini-cli
date: 2026-04-21
ready: true
feature: 2026-04-20-better-errors
scope:
  - biscuit-terminal/lib/src/errors/block_error.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/types.rs
  - darkmatter/lib/src/markdown/reference/errors.rs
  - darkmatter/lib/src/editor/mod.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/cli/src/main.rs
  - darkmatter/lib/tests/error_snapshots/*
---

# Better Errors — Review 2

## Summary

This second review confirms that all suggestions from the prior review (`review-1.md`) have been successfully implemented. The "Better Errors" feature is now structurally complete, idiomatically aligned with the technical design, and provides high-signal, actionable feedback across the entire Darkmatter ecosystem.

The transition from "good" to "great" error blocks has been achieved by enriching the core error enums with the missing structured context identified in the previous turn.

---

## 1. Resolution of Prior Review Suggestions

### 1.1 Schema Enrichments (P1) - RESOLVED

All high-value enrichments that were previously identified as "drift" from the technical design have now landed:

- **TransclusionError::CycleDetected**: Now carries `Vec<(PathBuf, usize)>` and renders the per-hop line numbers in the chain.
- **TransclusionError::InvalidReference**: Now includes `source_file` and `directive_kind`.
- **ConditionError::Parse**: Now includes a `span: Range<usize>` and utilizes a `caret_marker` in the rendered block to pinpoint parsing failures.
- **PageBlockError::UnterminatedBlock**: Now echoes the `opening_text` and indicates `file_ends_at_line`.
- **ReferenceError::ParseDirective**: Now includes `source_file`, `directive_text`, and `caret_col`, with caret rendering support.
- **NormalizationError**: Structure issues are now handled with better context.
- **EditorError**: Fully restructured with specific variants for `NonZeroExit`, `Missing`, `LaunchFailed`, and `Io` with an `operation` tag.

### 1.2 Correctness & Ergonomics - RESOLVED

- **`strip_ansi` Duplication**: All hand-rolled duplicates have been removed in favor of the canonical `biscuit_terminal::utils::escape_codes::strip_escape_codes`.
- **`as_block_error` Registry**: The downcast registry in `darkmatter/lib/src/markdown/errors/mod.rs` is fully populated and tested.
- **`MarkdownError` Delegation**: The delegation strategy correctly prioritizes inner block rendering while maintaining a clean, no-noise output for promoted errors.
- **Snapshot Coverage**: The `error_snapshots` test suite now provides comprehensive coverage across all error families, including the priority variants identified in the design.

---

## 2. Technical Observations

### 2.1 Performance
The implementation uses `truncate_output` for shell execution failures, preventing terminal flooding while preserving the most relevant signal (first 20 lines / 2KB). The character-index iteration has been optimized as suggested.

### 2.2 Ergonomics
The use of `ErrorHeader` and `StatusBlockExt` across all implementations ensures a consistent visual language. The title line pattern (`<b>ErrorName:</b> <b>Title</b>`) is strictly followed, and hints are actionable and imperative.

---

## 3. Verdict

**Ready for production.**

The feature meets and exceeds the requirements set out in the specification. The error reporting is now a first-class citizen of the Darkmatter experience, providing developers and users with the exact context needed to resolve issues without manual investigation.

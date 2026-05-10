---
ready: false
agent: gemini
model: ""
---

# Review: Good Errors (2026-05-08)

I have performed a comprehensive review of the "Good Errors" feature implementation based on the [specification](./spec.md) and the [execution plan](./plan.md).

## Executive Summary

The feature is **NOT ready for production**. 

While Phase 1 (Reference Implementation) is successfully established—introducing the new `biscuit-terminal` APIs, the `SourceContext` type, and a high-quality migration for `PageBlockError::UnterminatedBlock`—the project has stalled before completing Phase 2 (The Sweep). Almost all other error variants in the `darkmatter` package remain in their "legacy" state, providing poor ergonomics and lacking the rich source awareness designed in the spec.

## Findings

### High Severity

#### 1. Incomplete Phase 2 (The Sweep)
The vast majority of error variants that originate from source files have NOT been migrated to carry or render `SourceContext`. 
- **Affected Areas:** `TransclusionError`, `ReferenceError`, `LinkError`, `ImageRefError`, `StylesheetError`, and most of `PageBlockError`.
- **Impact:** Users receiving these errors still see "legacy" reports without clickable file paths, frontmatter snapshots, or gutter-aware source excerpts.
- **Verification:** I inspected `darkmatter/lib/src/markdown/compose/transclusion/types.rs` and `darkmatter/lib/src/render/link.rs`; both continue to use old-style `body(format!(...))` calls and legacy fields like `line: usize`.

#### 2. SourceContext Name Collision
There is a major architectural name collision that complicates the migration.
- **File:** `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
- **Issue:** This file defines its own `pub struct SourceContext { file, url }`. This conflicts with `biscuit_terminal::errors::SourceContext`.
- **Requirement:** This local struct should be deleted and replaced with the terminal-provided `SourceContext` during the Phase 2 migration.

#### 3. Misleading Snapshots
The `darkmatter/lib/tests/error_snapshots/snapshots/` directory contains dozens of snapshots for transclusion and render errors.
- **Issue:** These snapshots represent the OLD rendering style (verified by reading `error_snapshots__transclusion__cycle_detected.snap`).
- **Risk:** Their presence in the repository gives a false impression of feature completeness. They must be updated to reflect the rich rendering style once Phase 2 is actually implemented.

### Medium Severity

#### 4. Test Rigor Gap (Level 2)
The feature defines several user-observable behaviors that are currently only verified at Level 1 (and even then, only partially).
- **Observable Behaviors:**
    - OSC 8 Clickable Hyperlinks in error headers.
    - Gutter markers (`>`) and line numbers in excerpts.
    - Dimmed foreground and 2-space indentation for fenced code blocks in `Prose`.
- **Current Verification:** `darkmatter/lib/tests/error_snapshots/helpers.rs` uses `strip_ansi`, which removes exactly the bytes needed to verify colors and hyperlinks.
- **Requirement:** At least one Level 2 test (running in a real terminal like WezTerm/Kitty and capturing with `--escapes` or `--ansi`) must be added to verify that the rich rendering actually works in the wild.

#### 5. Manual Snippet Rendering in `MarkdownError`
The leaf helper `frontmatter_parse_block` in `darkmatter/lib/src/markdown/errors/blocks.rs` continues to implement its own manual snippet rendering.
- **Issue:** It does not use `SourceContext::excerpt_prose`.
- **Requirement:** Migrating this to `SourceContext` would ensure gutter style consistency across all Darkmatter errors.

## Ergonomics & Performance

- **Prose Tag Parsing:** The change to `StatusBlock::body(impl Into<Vec<Prose>>)` is excellent. It allows `body(format!(...))` to work safely while ensuring tags are never leaked as literal text.
- **Lazy Content:** The use of `Arc<str>` in `SourceContext` is a good performance choice, avoiding large string clones when generating errors for large documents.

## Recommendations

1. **Finish Phase 2:** Migrate all remaining error types to `SourceContext`. This is the single biggest gap.
2. **Resolve the Collision:** Delete the local `SourceContext` in `transclusion/types.rs` and unify on the `biscuit-terminal` version.
3. **Upgrade Test Helpers:** Modify or add a test helper that preserves ANSI/OSC8 for at least some snapshots, and add one Level 2 integration test.
4. **Update Documentation:** Complete the Phase 2 documentation tasks, including the `.claude/skills/darkmatter/errors.md` update.

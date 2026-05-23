# Render Tree Implementation Review - May 19, 2026

## Executive Summary

The render-tree implementation in the `renderable`, `darkmatter`, and `biscuit-terminal` crates has been reviewed against the requirements defined in `@renderable/features/2026-05-19-pushing-toward-ir/approved-render-tree-functionality.md`. 

The implementation **successfully meets all approved requirements**. All ten feature requests (RT-COMPOSE-001 through RT-TWOCOLUMN-002) are fully implemented in the tree infrastructure and renderers, backed by comprehensive unit tests.

## Feature-by-Feature Analysis

| ID | Feature | Status | Notes |
|:---|:---|:---|:---|
| **RT-COMPOSE-001** | Explicit no-separator sequence rendering | **PASS** | Implemented as `SequenceJoin::None` hint on `NodeKind::Root`. Honored by Terminal, Markdown, and Browser renderers. |
| **RT-FILESYSTEM-001** | Typed list marker policy | **PASS** | `ListMarkerPolicy` supports `Default`, `None`, and `TreeConnectors`. Terminal renderer correctly infers tree geometry; other renderers degrade gracefully. |
| **RT-PROGRESS-001** | Browser rendering for progress hints | **PASS** | Emits semantic HTML with `role="progressbar"`, ARIA attributes, and stable classes. Correctly lowers color and layout. |
| **RT-PROGRESS-002** | MarkdownPlus rendering for progress hints | **PASS** | Emits the same semantic HTML as Browser when using `MarkdownDialect::MarkdownPlus`. |
| **RT-TABLE-001** | Typed table title/caption hint | **PASS** | Implemented as `table_title` hint. Renders as `<caption>` in Browser, above border in Terminal, and as plain text in Markdown. |
| **RT-TABLE-002** | Markdown-safe table cell serialization | **PASS** | Applies `\|` escaping and `<br>` normalization inside `TableCell` descendants. Normalizes `SoftBreak` to space. |
| **RT-TEXTBLOCK-001** | Browser lowering for `Style` | **PASS** | Full coverage for color, background, bold, italic, strikethrough, underline, dim, and blink. |
| **RT-TODO-001** | Typed task-state hints | **PASS** | `TaskState` (5 states) honored by Terminal renderer with state-specific markers (Nerd Font/ASCII/No-color). Markdown/Browser use portable GFM fallbacks. |
| **RT-TWOCOLUMN-001** | Browser CSS lowering for `ColumnsHints` | **PASS** | Lowers to Flexbox CSS with fixed/percent widths and custom gaps. |
| **RT-TWOCOLUMN-002** | MarkdownPlus HTML lowering for `ColumnsHints` | **PASS** | Emits Flexbox HTML container in `MarkdownPlus` dialect; preserves sequential fallback in plain Markdown. |

## Observations and Gaps

### 1. Component Migration Status
While the tree infrastructure and renderers are fully feature-complete according to the approved requirements, the migration of existing structural components is still in its early stages as planned:
*   **`BlockQuote`**: Successfully migrated to `TreeRenderable` with a parity gate.
*   **`Todo`, `FileSystem`, `Table`, `Section`, `List`**: These components have NOT yet been updated to implement `TreeRenderable`. The renderer features supporting them (like `TaskState` or `ListMarkerPolicy`) are verified using synthetic trees in tests.
*   **`Prose`**: Intentionally omitted from the tree model (as documented in `tree-rendering.md`), resulting in some lossy styling when components like `BlockQuote` extract text from a `Prose` child.

### 2. Test Coverage
Test coverage is exceptionally high (400+ tests across the related crates). Specific tests were found for:
*   Nested `TreeConnectors` with continuation lines in Terminal.
*   Mixed inline/block children in `SequenceJoin::None` for Markdown.
*   Validation of all new hint types against their permitted `NodeKind` targets.
*   Color depth and Nerd Font detection branches for `TaskState` markers.

### 3. Fidelity and Degradation
The renderers correctly implement the `Strict` / `Warn` / `Lossy` policy:
*   `ListMarkerPolicy::TreeConnectors` degrades to `list-style:none` in Browser with a diagnostic.
*   `TaskState::InProgress` degrades to `- [ ]` in Markdown.
*   `Style` on block nodes is lowered to CSS to maintain HTML validity.

## Recommendations

1.  **Proceed with Component Migration**: Now that the renderer infrastructure is proven, follow the roadmap in `tree-rendering.md` to migrate `Section`, `List`, `Table`, and `FileSystem` to `TreeRenderable` one by one, each with its own parity gate.
2.  **Migrate `Todo`**: Given that `TaskState` infrastructure is ready, `Todo` is an ideal next candidate for migration to unify its bespoke and tree rendering paths.
3.  **Monitor Performance**: As the tree model uses owned `RenderNode` structures and `serde` serialization, monitor the cost profile during `darkmatter` migration, especially for large documents.

**Review Status: APPROVED**

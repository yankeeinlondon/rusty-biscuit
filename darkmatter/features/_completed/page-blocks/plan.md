# Page Blocks Implementation Plan

## Overview

Add `::block when="..."` / `::end-block` paired directives as a new Stage 2 compose pass that runs **before** block transclusion and frontmatter transclusion. False blocks are removed entirely (including any nested transclusion directives), true blocks have their wrapper lines stripped and body content preserved verbatim.

## Phase 1: Shared Condition Evaluator Extraction

**Goal:** Promote `transclusion/conditions.rs` into a Stage 2 shared module so both page blocks and transclusion use the same condition engine without cross-module coupling.

### Step 1.1: Create `compose/conditions.rs`

- Copy `transclusion/conditions.rs` to `compose/conditions.rs`
- Change the error type from `TransclusionError` to a new shared `ConditionError` enum:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ConditionError {
    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    Parse { expr: String, line: usize, message: String },
    #[error("Failed to evaluate condition '{expr}' at line {line}: {message}")]
    Eval { expr: String, line: usize, message: String },
}
```

- Update the `evaluate_condition()` signature to return `Result<bool, ConditionError>`
- Add `pub mod conditions;` to `compose/mod.rs`

### Step 1.2: Update transclusion to use shared conditions

- Remove `transclusion/conditions.rs`
- In `transclusion/mod.rs`, re-export or import from `super::conditions`
- Add `From<ConditionError> for TransclusionError` impl so existing transclusion call sites compile unchanged
- Verify: `just test` in darkmatter passes with no behavioral changes

## Phase 2: Types and Error Definitions

**Goal:** Define the data model for page blocks.

### Step 2.1: Create `compose/page_blocks/types.rs`

```rust
/// Parsed options from a `::block` directive line.
pub struct PageBlockOptions {
    pub when_expr: Option<String>,
    pub unknown_options: Vec<String>,
}

/// A parsed page block region with exact byte spans.
pub struct PageBlockRegion {
    /// Full span including `::block` and `::end-block` lines.
    pub span: std::ops::Range<usize>,
    /// Body span excluding wrapper lines.
    pub body_span: std::ops::Range<usize>,
    /// 1-based line number of the `::block` directive.
    pub start_line: usize,
    /// 1-based line number of the `::end-block` directive.
    pub end_line: usize,
    /// Parsed options from the start directive.
    pub options: PageBlockOptions,
    /// Nested child blocks.
    pub children: Vec<PageBlockRegion>,
}
```

### Step 2.2: Create `compose/page_blocks/mod.rs`

- Declare submodules: `pub mod types; pub mod parser; pub mod engine;`
- Re-export key types

### Step 2.3: Define `PageBlockError`

Add to `types.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PageBlockError {
    #[error("Failed to parse page block at line {line}: {message}")]
    ParseDirective { line: usize, message: String },
    #[error("Unmatched ::end-block at line {line}")]
    UnmatchedEnd { line: usize },
    #[error("Unterminated ::block starting at line {line}")]
    UnterminatedBlock { line: usize },
    #[error("{0}")]
    Condition(#[from] super::conditions::ConditionError),
}
```

### Step 2.4: Wire `PageBlockError` into `ComposeError`

- Add `PageBlock(PageBlockError)` variant to the main `ComposeError` enum (or `MarkdownError` — whichever is the pipeline's top-level error)
- Add the corresponding `From` impl

## Phase 3: Parser

**Goal:** Parse `::block` / `::end-block` directives into a nested `PageBlockRegion` tree.

### Step 3.1: Create `compose/page_blocks/parser.rs`

Implement `pub fn parse_page_blocks(content: &str) -> Result<Vec<PageBlockRegion>, PageBlockError>`:

1. Call `find_code_regions(content)` to get fenced-code byte ranges
2. Iterate lines with byte offsets (use `content.as_bytes()` tracking or `lines()` with cumulative offset)
3. For each line not inside a code region:
   - If trimmed line starts with `::block`:
     - Parse attributes using a `Cursor` (same pattern as `transclusion/parser.rs`)
     - Recognized attribute: `when` → store in `PageBlockOptions.when_expr`
     - Unknown attributes → collect in `PageBlockOptions.unknown_options`
     - Push onto a stack: `(start_byte, start_line, options, children_so_far)`
   - If trimmed line starts with `::end-block`:
     - Validate no trailing non-whitespace content
     - Pop from the stack
     - If stack is empty → return `UnmatchedEnd` error
     - Compute `span` (full) and `body_span` (between wrapper lines)
     - Build `PageBlockRegion` with popped data
     - If stack is non-empty → push region as child of current top
     - If stack is empty → push region to top-level results
4. After all lines: if stack is non-empty → return `UnterminatedBlock` for the deepest open block
5. Return the top-level `Vec<PageBlockRegion>`

### Step 3.2: Parser unit tests

Add `#[cfg(test)] mod tests` in `parser.rs` with tests for:

1. Single valid block → correct spans, line numbers, options
2. Multiple sibling blocks → correct ordering and independent spans
3. Nested blocks → children populated correctly, spans nested
4. Block inside fenced code → ignored entirely
5. `::end-block` without opener → `UnmatchedEnd` error
6. EOF with unclosed block → `UnterminatedBlock` error
7. Unknown attributes captured in `unknown_options`
8. `::block` with no attributes → `when_expr` is `None`
9. `::end-block` with trailing non-whitespace → error
10. Empty block body → valid, body_span is empty range

## Phase 4: Engine (Rendering)

**Goal:** Walk the parsed region tree, evaluate conditions, and produce output content.

### Step 4.1: Create `compose/page_blocks/engine.rs`

Implement `pub fn render_page_blocks(content: &str, regions: &[PageBlockRegion], state: &EffectiveState, report: &mut ComposeReport) -> Result<String, PageBlockError>`:

1. Sort top-level regions by `span.start` (they should already be in order from the parser)
2. Initialize `cursor = 0` and `output = String::new()`
3. For each top-level region:
   - Append `content[cursor..region.span.start]` to output (literal text before this block)
   - Evaluate `when_expr` via `conditions::evaluate_condition()` (if `None`, treat as `true`)
   - If true:
     - Recursively render the body: call a helper that processes `region.children` against `content[region.body_span]`
     - Append rendered body to output
     - Increment `report.page_blocks_rendered`
   - If false:
     - Append nothing
     - Increment `report.page_blocks_skipped`
   - Set `cursor = region.span.end`
4. Append `content[cursor..]` (trailing content after last block)
5. Return output

**Recursive helper** for nested blocks:

```rust
fn render_body(
    content: &str,
    body_span: &Range<usize>,
    children: &[PageBlockRegion],
    state: &EffectiveState,
    report: &mut ComposeReport,
) -> Result<String, PageBlockError>
```

This follows the same cursor-walk pattern but scoped to the body span, with children as the regions to process.

### Step 4.2: Engine unit tests

Add tests for:

1. `when` true → body content preserved, wrapper lines removed
2. `when` false → entire block removed
3. `when` omitted → treated as true
4. Frontmatter variable resolution: `when="state == 'foo'"`
5. Environment variable resolution: `when="env.AGENT"`
6. True outer, false inner → outer body kept minus inner block
7. False outer → all nested content removed (inner blocks never evaluated)
8. Content before, between, and after blocks preserved byte-for-byte
9. Report counters incremented correctly
10. Condition parse error → `PageBlockError::Condition`

## Phase 5: Pipeline Integration

**Goal:** Wire page blocks into the Stage 2 pipeline.

### Step 5.1: Add `page_blocks` to `Stage2Stages`

In `compose/types.rs`:

```rust
pub struct Stage2Stages {
    pub page_blocks: bool,          // NEW
    pub block_transclusion: bool,
    pub fm_transclusion: bool,
}
```

- Update `Default` impl: `page_blocks: true`
- Update `Stage2Stages::none()`: `page_blocks: false`
- Add `Stage2Stages::only_page_blocks()` convenience constructor

### Step 5.2: Add report fields

In `ComposeReport`:

```rust
pub page_blocks_rendered: usize,
pub page_blocks_skipped: usize,
```

- Update `Default` impl
- Update `has_changes()` to include `page_blocks_rendered > 0`
- Update `summary()` to include page block stats when non-zero

### Step 5.3: Add `run_page_blocks_stage` to the composer

In `compose/mod.rs`:

```rust
impl MarkdownComposer {
    fn run_page_blocks_stage(
        &mut self,
        state: &EffectiveState,
        _options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        let regions = page_blocks::parser::parse_page_blocks(&self.content)?;
        if regions.is_empty() {
            return Ok(());
        }

        // Warn for unknown options
        for region in &regions {
            warn_unknown_options(region, report);
        }

        self.content = page_blocks::engine::render_page_blocks(
            &self.content, &regions, state, report,
        )?;
        Ok(())
    }
}
```

### Step 5.4: Insert into Stage 2 execution order

In the `run_compose_pipeline_internal` method (or wherever Stage 2 is orchestrated), add **before** block transclusion:

```rust
// Stage 2
if options.stage2.page_blocks {
    self.run_page_blocks_stage(&effective_state, &options, &mut report)?;
}
if options.stage2.block_transclusion {
    self.run_block_transclusion_stage(...)?;
}
if options.stage2.fm_transclusion {
    self.run_frontmatter_transclusion_stage(...)?;
}
```

### Step 5.5: Register the module

- Add `pub mod page_blocks;` to `compose/mod.rs`
- Add `mod page_blocks` to `darkmatter/lib/src/markdown/compose/mod.rs` module declarations

## Phase 6: Pipeline Integration Tests

**Goal:** Verify end-to-end behavior through the full compose pipeline.

### Step 6.1: Create integration test file

Add tests in an appropriate test location (either inline in `compose/mod.rs` tests or a dedicated test file) covering:

1. **Ordering:** Page blocks run before transclusion
   - A `::file` directive inside a false `::block` must not be resolved
   - A `::file` directive inside a true `::block` must be resolved normally

2. **Transcluded child documents:** A transcluded markdown file containing `::block` directives evaluates them during its own recursive compose

3. **Coexistence with Stage 1:** Interpolation output from Stage 1 is visible to page block conditions (since page blocks run in Stage 2)

4. **Report accuracy:** `page_blocks_rendered`, `page_blocks_skipped`, and warnings all populated correctly through the full pipeline

5. **Stage toggle:** `Stage2Stages { page_blocks: false, .. }` leaves `::block` directives as literal text

## Phase 7: Regression and Edge Case Tests

### Step 7.1: Edge case coverage

1. Code fence containing `::block` → treated as literal text
2. Empty block body → valid, produces empty output when true
3. Block at very start of file (first line)
4. Block at very end of file (last line, `::end-block` is final line)
5. Adjacent blocks with no content between them
6. Block with only whitespace body
7. Deeply nested blocks (3+ levels)
8. Multiple unknown attributes → all captured in warnings

## Implementation Order Summary

| Phase | Description | Dependencies | Estimated Complexity |
|-------|-------------|-------------|---------------------|
| 1 | Shared condition extraction | None | Low — mostly moving code |
| 2 | Types and error definitions | Phase 1 | Low — struct/enum definitions |
| 3 | Parser | Phase 2 | Medium — paired-block parsing with nesting |
| 4 | Engine | Phase 2, 3 | Medium — recursive rendering |
| 5 | Pipeline integration | Phase 1-4 | Low — wiring existing pieces |
| 6 | Integration tests | Phase 5 | Medium — end-to-end scenarios |
| 7 | Regression tests | Phase 5 | Low — edge cases |

## Files Created/Modified

### New files:
- `darkmatter/lib/src/markdown/compose/conditions.rs`
- `darkmatter/lib/src/markdown/compose/page_blocks/mod.rs`
- `darkmatter/lib/src/markdown/compose/page_blocks/types.rs`
- `darkmatter/lib/src/markdown/compose/page_blocks/parser.rs`
- `darkmatter/lib/src/markdown/compose/page_blocks/engine.rs`

### Modified files:
- `darkmatter/lib/src/markdown/compose/mod.rs` — add module declarations, `run_page_blocks_stage`, Stage 2 ordering
- `darkmatter/lib/src/markdown/compose/types.rs` — `Stage2Stages` field, `ComposeReport` fields, `ComposeWarning` usage
- `darkmatter/lib/src/markdown/compose/transclusion/mod.rs` — re-export from shared conditions
- `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs` — removed (promoted to shared)
- Error enum file (wherever `ComposeError` or `MarkdownError` is defined) — add `PageBlock` variant

### Not modified:
- No changes to Darkmatter's public Markdown AST or renderer types
- No changes to Stage 1 pipeline
- No changes to CLI or binary entry points

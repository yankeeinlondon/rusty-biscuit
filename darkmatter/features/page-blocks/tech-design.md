# Page Blocks Tech Design

This document defines the implementation-ready technical design for the `page-blocks` feature in Darkmatter. It is derived from:

- `darkmatter/features/page-blocks/spec.md`
- `darkmatter/docs/preparation/page-blocks.md`
- the current compose pipeline in `darkmatter/lib/src/markdown/compose/`
- the existing Stage 2 transclusion implementation and shared condition semantics

The design goal is to add conditional, in-document content regions that compose cleanly with the current Stage 1 and Stage 2 pipeline without inventing a parallel expression or state model.

## Purpose

Page blocks let authors mark regions of a Markdown document that are included or omitted during composition based on a `when` expression:

```md
## My Section

::block when="state == 'foo'"

Foo is the best

::end-block

::block when="env.AGENT == 'claude'"

Claude-specific content

::end-block
```

The feature is intentionally narrow in v1:

- page blocks are a composition-time feature, not a rendering-time feature
- `when` reuses the same expression grammar and truthiness rules already used by transclusion
- the body of a block remains ordinary Markdown and can contain headings, lists, code fences, interpolation output, and transclusion directives

## Scope

In scope:

1. `::block ...` / `::end-block` parsing
2. `when="..."` evaluation using existing Darkmatter condition semantics
3. Stage 2 integration in the compose pipeline
4. nested page blocks
5. interaction with transclusion, interpolation, and environment-driven composition
6. diagnostics for malformed or unterminated blocks
7. test coverage for parser, evaluator, and pipeline behavior

Out of scope for v1:

1. attributes beyond `when`
2. block-specific rendering wrappers or metadata
3. inline page blocks embedded mid-paragraph
4. page-block-aware whitespace cleanup beyond literal span replacement
5. new expression syntax or a second condition evaluator

## Current Baseline

Darkmatter already has:

1. a Stage 1 pipeline for replacement, interpolation, TOC linking, shell expansion, cleanup, and normalization
2. a Stage 2 pipeline for block transclusion and frontmatter transclusion
3. a shared `EffectiveState` model that merges frontmatter, external state, overrides, runtime context, and environment variables
4. a working condition parser and evaluator in the transclusion implementation
5. directive parsing utilities that skip fenced code blocks

Darkmatter does not yet have:

1. paired block directives that enclose body content
2. a Stage 2 pass that can remove local content before nested directives are processed
3. page-block-specific diagnostics or report counters

## Primary Recommendation

Add page blocks as a new Stage 2 pass that runs before block transclusion and frontmatter transclusion.

The recommended Stage 2 order becomes:

1. page blocks
2. block transclusion
3. frontmatter transclusion

This ordering is the key design choice.

Rationale:

1. if a page block evaluates to false, any nested `::file`, `::code`, or future expensive directives inside that block should never execute
2. conditions should evaluate against the document state that exists after Stage 1 preparation but before new Stage 2 content is injected
3. transcluded child documents can still use page blocks because recursive composes will run the same Stage 2 order

## Assumptions

The spec leaves several details implicit. This design makes the following assumptions:

1. `::block` and `::end-block` are composition directives and are only recognized when they appear on their own logical lines, ignoring indentation
2. `when` is optional; a page block without `when` is treated as enabled
3. the example `when="state == 'foo'"` refers to a normal frontmatter key named `state`, not a special `state.` namespace
4. code fences remain a hard boundary; directive-looking text inside fenced code blocks is never interpreted
5. nested page blocks are allowed because the paired-block parser naturally supports them and future block features will benefit from that structure

## Syntax Contract

Supported start directive:

```md
::block when="env.AGENT == 'claude'"
```

Supported end directive:

```md
::end-block
```

Rules:

1. `::block` accepts zero or more `key=value` attributes
2. v1 recognizes only `when`
3. unknown attributes are ignored with a warning, matching current transclusion option behavior
4. `::end-block` accepts no attributes or trailing content beyond whitespace
5. directives are line-oriented; inline uses such as `Paragraph ::block when="x"` are not supported

## Parsing Model

### Overview

The feature should use a paired-block parser rather than a line-by-line replacement loop. Nested blocks are much easier to implement correctly if the parser builds an internal tree of block regions first and rendering happens in a second pass.

### Recommended module layout

```txt
darkmatter/lib/src/markdown/compose/
├── conditions.rs
├── page_blocks/
│   ├── mod.rs
│   ├── parser.rs
│   ├── types.rs
│   └── engine.rs
├── transclusion/
└── ...
```

### Shared condition evaluator

`transclusion/conditions.rs` should be promoted into a Stage 2 shared module such as `compose/conditions.rs`.

Reasons:

1. page blocks and transclusion must stay behaviorally identical for `when`
2. a transclusion-specific error type is the wrong abstraction for page blocks
3. this avoids drift if function names, truthiness rules, or variable resolution semantics change later

### Parser algorithm

The parser should:

1. compute fenced-code regions with the existing `find_code_regions()` helper
2. scan the document line by line
3. recognize trimmed lines that begin with `::block` or `::end-block` and are not inside fenced code
4. parse `::block` attributes with the existing cursor-style key/value parser used by transclusion
5. maintain a stack of open blocks
6. emit a tree of parsed regions with exact byte spans for:
   - the full block span, including start and end directive lines
   - the body span, excluding the wrapper lines
   - the source line numbers for diagnostics

Malformed input should fail early:

1. `::end-block` without a matching opener
2. EOF with an unclosed block
3. malformed attributes or unterminated quoted values
4. trailing non-whitespace content on `::end-block`

## AST and Data Model Changes

This feature does not require changes to Darkmatter's public Markdown AST or renderer-facing types. The changes are confined to compose-time data structures.

Recommended internal types:

```rust
pub struct PageBlockOptions {
    pub when_expr: Option<String>,
    pub unknown_options: Vec<String>,
}

pub struct PageBlockRegion {
    pub span: std::ops::Range<usize>,
    pub body_span: std::ops::Range<usize>,
    pub start_line: usize,
    pub end_line: usize,
    pub options: PageBlockOptions,
    pub children: Vec<PageBlockRegion>,
}
```

Recommended shared error additions:

```rust
pub enum ConditionError {
    Parse { expr: String, line: usize, message: String },
    Eval { expr: String, line: usize, message: String },
}
```

Recommended page-block error type:

```rust
pub enum PageBlockError {
    ParseDirective { line: usize, message: String },
    UnmatchedEnd { line: usize },
    UnterminatedBlock { line: usize },
    Condition(ConditionError),
}
```

Recommended public pipeline changes:

```rust
pub struct Stage2Stages {
    pub page_blocks: bool,
    pub block_transclusion: bool,
    pub fm_transclusion: bool,
}
```

```rust
pub struct ComposeReport {
    pub page_blocks_rendered: usize,
    pub page_blocks_skipped: usize,
    // existing fields...
}
```

`Stage2Stages::default()` should enable `page_blocks`, and a convenience constructor such as `Stage2Stages::only_page_blocks()` should be added for focused tests.

## Composition and Rendering Behavior

### Execution model

The new Stage 2 pass should operate on the fully prepared content produced by Stage 1. The engine should render the document by walking the parsed block tree and splicing kept content back into the source string.

Algorithm:

1. parse page blocks into a region tree
2. walk top-level regions in source order
3. copy literal source text that appears before each region
4. evaluate the region's `when` condition, if present
5. if true:
   - render the region body
   - recursively apply the same logic to nested child regions
   - append the rendered body without the wrapper lines
6. if false:
   - append nothing for that region
7. copy trailing source text after the last region

This gives exact control over nested blocks and preserves source text outside the directives verbatim.

### Interaction with transclusion

If a page block body contains transclusion directives:

1. a true block leaves those directives in the content stream
2. a false block removes them before the transclusion stage sees them

This is the main reason page blocks belong before transclusion in Stage 2.

### Interaction with recursive composition

Transcluded markdown files already run through recursive transforms. With this design:

1. child documents evaluate their own page blocks during their transform
2. child block conditions use the child's effective state, which already includes inherited parent state according to current composition rules
3. the parent document does not need a special second pass for child page blocks

### Whitespace behavior

The page-block stage should perform literal span replacement only:

1. a true block removes wrapper lines and keeps the body exactly as written
2. a false block removes the entire span exactly

The stage should not try to invent cleanup heuristics around removed blocks. That keeps the behavior predictable and avoids fighting author formatting. If later composition workflows need post-Stage-2 cleanup, that should be a separate pipeline decision rather than hidden inside page blocks.

## `when` Evaluation Semantics

Page blocks must use the same expression grammar and truthiness rules as transclusion `when`.

That means:

1. variables resolve through `EffectiveState`
2. frontmatter keys are referenced directly by name, such as `state`
3. environment variables are referenced as `env.NAME`
4. context values remain available through existing paths such as `ctx.today` if that is already supported by the shared evaluator
5. missing values resolve to `null` and are falsey
6. an empty string is falsey, so `!env.AGENT` is true when `AGENT` is unset or empty
7. comparison, ternary, fallback, and helper functions behave exactly as they do in transclusion

Examples:

```md
::block when="state == 'foo'"
Content for foo
::end-block

::block when="env.AGENT"
Content shown only when AGENT is set
::end-block
```

If `when` is omitted, the block is treated as enabled. This keeps the syntax future-compatible for non-conditional block features while remaining intuitive today.

## Error Handling and Validation

### Hard errors

The following should be fatal regardless of `fail_fast`:

1. malformed page-block syntax
2. unmatched `::end-block`
3. unterminated blocks
4. invalid `when` expressions
5. evaluation failures in `when`

These errors indicate authoring mistakes in the composition structure itself. Silently continuing would make the resulting document hard to trust.

### Warnings

The following should be non-fatal warnings recorded in `ComposeReport`:

1. unknown `::block` attributes

This matches the current transclusion behavior and leaves room for future attributes without making typo handling too permissive.

### Error messages

Diagnostics should include:

1. the source line number of the opening directive when possible
2. the offending expression for condition failures
3. whether the failure happened during parsing or evaluation

Representative messages:

- `Failed to parse page block at line 12: unknown token after ::end-block`
- `Unterminated ::block starting at line 8`
- `Failed to evaluate page-block condition 'env.AGENT ==' at line 8: expected expression`

## Public API Integration

`compose/mod.rs` should add a new Stage 2 hook before `run_block_transclusion_stage(...)`:

```rust
impl MarkdownComposer {
    fn run_stage2(
        &mut self,
        effective_state: &EffectiveState,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        if options.stage2.page_blocks {
            self.run_page_blocks_stage(effective_state, options, report)?;
        }

        Ok(())
    }
}
```

Recommended implementation shape:

```rust
impl MarkdownComposer {
    fn run_page_blocks_stage(
        &mut self,
        state: &EffectiveState,
        options: &ComposeOptions,
        report: &mut ComposeReport,
    ) -> MarkdownResult<()> {
        todo!()
    }
}
```

The `options` parameter is still useful even if v1 does not need many knobs, because it keeps the signature aligned with other stages and leaves room for future block attributes.

## Test Strategy

### Parser tests

Add focused unit tests for:

1. a single valid block
2. multiple sibling blocks
3. nested blocks
4. blocks inside fenced code being ignored
5. unmatched `::end-block`
6. missing `::end-block`
7. unknown attributes being captured for warnings

### Condition and rendering tests

Add unit tests for:

1. `when` resolving against frontmatter
2. `when` resolving against `env.*`
3. omitted `when` behaving as true
4. true outer block with false nested block
5. false outer block skipping nested content entirely
6. literal content before, between, and after blocks being preserved byte-for-byte

### Pipeline integration tests

Add compose-level tests for:

1. page blocks running before transclusion
2. transclusion inside a false page block not executing
3. transcluded child markdown evaluating its own page blocks
4. page blocks coexisting with interpolation output produced in Stage 1
5. report counters and warnings being populated correctly

### Regression tests

Add explicit regression coverage for:

1. code fences containing `::block`
2. empty block bodies
3. blocks at start or end of file
4. Windows-style line endings if the existing transform tests cover CRLF-sensitive behavior

## Rollout and Compatibility

This is a backward-incompatible interpretation change for documents that already contain literal lines beginning with `::block` and `::end-block` outside code fences. The risk appears low, but the design should still provide an escape hatch.

Recommended compatibility strategy:

1. enable page blocks by default in `Stage2Stages`
2. allow callers to disable them with `Stage2Stages { page_blocks: false, .. }`
3. document the reserved directive names in Darkmatter docs
4. add examples showing both frontmatter-driven and environment-driven usage

No migration is required for callers already using the transform pipeline, because the feature is additive at the API level.

## Open Questions

1. Should unknown `::block` attributes remain warnings forever, or should Darkmatter eventually support a strict mode that upgrades them to errors?
2. Should a future pipeline add an optional post-Stage-2 cleanup pass for users who want conditional composition to collapse extra blank lines automatically?
3. If future page-block attributes introduce wrappers or metadata, should the internal `PageBlockRegion` become a more general Stage 2 region tree shared with other paired directives?

## Summary

The cleanest implementation is a new Stage 2 `page_blocks` pass that:

1. parses `::block` / `::end-block` pairs into a nested region tree
2. evaluates `when` with the same shared condition engine used by transclusion
3. removes false blocks before transclusion runs
4. preserves body content exactly for true blocks
5. integrates with the existing compose report and stage toggles

That approach keeps the feature narrow, predictable, and compatible with Darkmatter's existing composition architecture.

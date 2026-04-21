---
spec: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/page-blocks/spec.md
feature: page-blocks
tech_design: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/page-blocks/tech-design.md
last_updated: 2026-03-16
plan: darkmatter/features/page-blocks/plan.md
implement_complete: false
implementation_files: darkmatter/lib/src/markdown/transform/conditions.rs, darkmatter/lib/src/markdown/transform/page_blocks/mod.rs, darkmatter/lib/src/markdown/transform/page_blocks/types.rs, darkmatter/lib/src/markdown/transform/page_blocks/parser.rs, darkmatter/lib/src/markdown/transform/page_blocks/engine.rs, darkmatter/lib/src/markdown/transform/transclusion/conditions.rs, darkmatter/lib/src/markdown/transform/mod.rs, darkmatter/lib/src/markdown/transform/types.rs, darkmatter/lib/src/markdown/types.rs
---

## Tech Design for page-blocks Complete

**Timestamp:** 2026-03-16 13:25:46 PDT

The design adds `page-blocks` as a new Stage 2 transform pass that runs before block and frontmatter transclusion. Its core architectural choice is to parse `::block ...` / `::end-block` pairs into a nested region tree, then render by splicing kept body spans back into the source. This lets false blocks remove their entire content, including any nested transclusion directives, before later Stage 2 work executes.

Behaviorally, `when` reuses the existing transclusion condition engine and `EffectiveState`, so frontmatter keys, `env.*`, truthiness, comparisons, and helper functions all stay consistent with current Darkmatter semantics. True blocks strip only their wrapper lines and preserve body content verbatim; false blocks remove the whole span. The design also proposes shared condition errors, a dedicated `page_blocks` stage toggle, and new report counters for rendered/skipped blocks.

Testing is centered on parser correctness, nested block rendering, fenced-code immunity, `when` evaluation against frontmatter and environment state, Stage 2 ordering relative to transclusion, and regression cases like empty blocks and CRLF input. The main compatibility risk is that literal `::block` / `::end-block` lines outside code fences will become reserved syntax, though the design keeps an escape hatch via a stage toggle. Open questions remain around whether unknown attributes should stay warnings, whether post-Stage-2 cleanup should ever collapse blank lines, and whether future paired directives should share a more general region-tree abstraction.

## Plan for page-blocks

**Timestamp:** 2026-03-16

The implementation plan organizes work into 7 phases:

1. **Phase 1 — Shared Condition Extraction:** Promote `transclusion/conditions.rs` to `transform/conditions.rs` with a new `ConditionError` type, so both page blocks and transclusion share the same condition engine without cross-module coupling. Transclusion updated to import from the shared location.

2. **Phase 2 — Types and Error Definitions:** Define `PageBlockOptions`, `PageBlockRegion` (with byte spans, line numbers, nested children), and `PageBlockError` (parse, unmatched end, unterminated, condition errors). Wire into the top-level `TransformError`.

3. **Phase 3 — Parser:** Build a paired-block parser (`parse_page_blocks`) that scans lines, respects fenced-code regions via `find_code_regions()`, maintains a stack for nesting, and emits a `Vec<PageBlockRegion>` tree. Includes 10 unit test cases covering valid blocks, nesting, code fence immunity, and error paths.

4. **Phase 4 — Engine:** Implement `render_page_blocks` that walks the region tree, evaluates `when` conditions via the shared evaluator, splices kept body content (recursive for nested blocks), and updates `TransformReport` counters. Includes 10 unit test cases for condition evaluation, nesting behavior, and report accuracy.

5. **Phase 5 — Pipeline Integration:** Add `page_blocks: bool` to `Stage2Stages`, add `page_blocks_rendered`/`page_blocks_skipped` to `TransformReport`, implement `run_page_blocks_stage` on `MarkdownTransformer`, and insert it as the first Stage 2 pass (before block transclusion and frontmatter transclusion).

6. **Phase 6 — Integration Tests:** End-to-end pipeline tests verifying Stage 2 ordering (false blocks prevent transclusion), recursive child document behavior, Stage 1 output visibility, report accuracy, and stage toggle bypass.

7. **Phase 7 — Regression Tests:** Edge cases including code fences with `::block`, empty bodies, blocks at file boundaries, adjacent blocks, deeply nested blocks, and multiple unknown attributes.

**New files (5):** `transform/conditions.rs`, `transform/page_blocks/{mod,types,parser,engine}.rs`

**Modified files (4-5):** `transform/mod.rs`, `transform/types.rs`, `transclusion/mod.rs`, `transclusion/conditions.rs` (removed), error enum file

### Phase 1: Shared Condition Evaluator Extraction

Created `transform/conditions.rs` with `ConditionError` enum and the full condition evaluator (previously in `transclusion/conditions.rs`). Updated `transclusion/conditions.rs` to delegate to the shared module with a `From<ConditionError> for TransclusionError` impl. Added `mod conditions;` to `transform/mod.rs`. All existing transclusion condition tests pass unchanged.

Files created: `darkmatter/lib/src/markdown/transform/conditions.rs`
Files modified: `darkmatter/lib/src/markdown/transform/transclusion/conditions.rs`, `darkmatter/lib/src/markdown/transform/mod.rs`

### Phase 2: Types and Error Definitions

Created `transform/page_blocks/` module with `types.rs` containing `PageBlockError`, `PageBlockOptions`, and `PageBlockRegion`. Created `mod.rs` declaring submodules and re-exports. Added `PageBlock(PageBlockError)` variant to `MarkdownError`. Added `pub mod page_blocks;` to `transform/mod.rs`.

Files created: `darkmatter/lib/src/markdown/transform/page_blocks/{mod.rs, types.rs}`
Files modified: `darkmatter/lib/src/markdown/types.rs`, `darkmatter/lib/src/markdown/transform/mod.rs`

### Phase 3: Parser

Implemented `parse_page_blocks()` in `parser.rs` — scans lines, skips fenced code regions, maintains a stack for nesting, and emits a `Vec<PageBlockRegion>` tree with exact byte spans. Validates `::block` is not confused with similar names (e.g., `::blockquote`). Includes 10 unit tests + 8 regression tests covering valid blocks, nesting, code fence immunity, error paths, edge cases (start/end of file, adjacent blocks, deeply nested, indented directives).

Files created: `darkmatter/lib/src/markdown/transform/page_blocks/parser.rs`

### Phase 4: Engine (Rendering)

Implemented `render_page_blocks()` in `engine.rs` — walks the parsed region tree, evaluates `when` conditions via the shared evaluator, recursively processes nested children, and splices kept body content. Includes 10 unit tests + 6 regression tests covering condition evaluation, nesting behavior, report counters, error propagation, empty blocks, and edge cases.

Files created: `darkmatter/lib/src/markdown/transform/page_blocks/engine.rs`

### Phase 5: Pipeline Integration

Added `page_blocks: bool` to `Stage2Stages` (default: `true`). Added `page_blocks_rendered`/`page_blocks_skipped` to `TransformReport` with `has_changes()` and `summary()` integration. Added `only_page_blocks()` convenience constructor. Implemented `run_page_blocks_stage()` on `MarkdownTransformer` with unknown-option warning support. Inserted as the first Stage 2 pass before block transclusion.

Files modified: `darkmatter/lib/src/markdown/transform/types.rs`, `darkmatter/lib/src/markdown/transform/mod.rs`

### Phase 6: Pipeline Integration Tests

Added 5 end-to-end pipeline tests to `transform/mod.rs::tests`: true block preservation, false block removal, coexistence with Stage 1 interpolation, report and warning population, and stage toggle bypass.

Files modified: `darkmatter/lib/src/markdown/transform/mod.rs`

### Phase 7: Regression and Edge Case Tests

Added parser regression tests: blocks at file start/end, adjacent blocks, whitespace-only body, 3-level nesting, multiple unknown attributes, directive name disambiguation, and indented directives. Added engine regression tests: empty body, file boundaries, adjacent mixed conditions, deep nesting all true, and no-blocks passthrough.

Files modified: `darkmatter/lib/src/markdown/transform/page_blocks/parser.rs`, `darkmatter/lib/src/markdown/transform/page_blocks/engine.rs`

## Implementation of page-blocks Complete

All 7 phases implemented successfully. Test results: 1388 passed, 1 failed (pre-existing `test_table_very_narrow_width` in unrelated terminal table module), 1 ignored. All page block tests pass.

New files (5):
- `darkmatter/lib/src/markdown/transform/conditions.rs`
- `darkmatter/lib/src/markdown/transform/page_blocks/mod.rs`
- `darkmatter/lib/src/markdown/transform/page_blocks/types.rs`
- `darkmatter/lib/src/markdown/transform/page_blocks/parser.rs`
- `darkmatter/lib/src/markdown/transform/page_blocks/engine.rs`

Modified files (4):
- `darkmatter/lib/src/markdown/transform/mod.rs`
- `darkmatter/lib/src/markdown/transform/types.rs`
- `darkmatter/lib/src/markdown/transform/transclusion/conditions.rs`
- `darkmatter/lib/src/markdown/types.rs`
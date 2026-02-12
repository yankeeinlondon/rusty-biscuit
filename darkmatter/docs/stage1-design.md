# Stage 1 Design: Preparation Transform Pipeline

## Skills Used

- rust
- darkmatter

## Purpose

Define a unified technical design for Stage 1 of the Darkmatter transform pipeline, implemented behind a new `Markdown::transform()` entry point.

Stage 1 includes:

1. Text Replacement
2. Frontmatter Interpolation
3. Markdown Cleaning
4. Normalization

This design aligns the existing docs and current code, while leaving room for later refactors as Stage 2+ features are added.

## Scope

In scope:

- A new transform orchestration API on `Markdown`
- Stage ordering and data flow
- Shared runtime/effective-state model for Stage 1
- Integration with existing cleanup/normalization code
- Error handling, reporting, and test strategy

Out of scope:

- Stage 2+ transforms (transclusion, summarization, consolidation, optimization features)
- Output rendering (`as_html`, `as_terminal`, `as_ast`)
- Full implementation details for every interpolation grammar edge case

## Current Baseline (Code Today)

Current capabilities in `darkmatter/lib/src/markdown/mod.rs`:

- `Markdown::cleanup(&mut self) -> &mut Self` delegates to `cleanup::cleanup_content`
- `Markdown::normalize(&self, target) -> Result<(Markdown, NormalizationReport), NormalizationError>`
- `Markdown::normalize_mut(&mut self, target) -> Result<NormalizationReport, NormalizationError>`
- `Markdown::relevel(&self, target) -> Result<(Markdown, i8), NormalizationError>`

Current gaps:

- No `Markdown::transform()` orchestration API yet
- Text replacement is documented but not implemented
- Frontmatter interpolation is documented but not implemented

Behavior notes relevant to Stage 1 planning:

- `cleanup` is already a robust formatting pass (event-stream based, table alignment, marker preservation)
- `normalize` currently performs uniform heading re-leveling; it returns a `NormalizationReport`
- `normalize` returns unchanged content when effective level adjustment is `0`

## Stage 1 Contract

`transform()` is the single entry point for Stage 1 preparation transforms. It is intentionally independent from output targets.

Pipeline order is fixed:

1. Text Replacement
2. Frontmatter Interpolation
3. Markdown Cleaning
4. Normalization

Rationale:

- Replacement runs before interpolation so raw tokens can be replaced before template evaluation.
- Interpolation runs before cleanup so generated text is included in cleanup formatting decisions.
- Cleanup runs before normalization so structural heading changes happen after layout cleanup.
- Normalization remains last to avoid downstream text rewrites changing heading markers again.

## Public API Design

Use a two-tier API:

```rust
impl Markdown {
    pub fn transform(&self) -> MarkdownResult<(Markdown, TransformReport)>;

    pub fn transform_with(
        &self,
        options: TransformOptions,
    ) -> MarkdownResult<(Markdown, TransformReport)>;

    pub fn transform_mut(
        &mut self,
        options: TransformOptions,
    ) -> MarkdownResult<TransformReport>;
}
```

Design intent:

- `transform()` gives an easy default pipeline path.
- `transform_with()` allows runtime state injection and stage controls.
- `transform_mut()` mirrors existing `_mut` patterns and avoids extra allocation at call sites.

## Transform Options and Report

```rust
#[derive(Debug, Clone)]
pub struct TransformOptions {
    pub state: Option<FrontmatterMap>,
    pub merge_strategy: MergeStrategy,
    pub stages: Stage1Stages,
    pub normalize_target: Option<HeadingLevel>,
    pub fail_fast: bool,
    pub context: TransformContext,
}

#[derive(Debug, Clone)]
pub struct Stage1Stages {
    pub text_replacement: bool,
    pub interpolation: bool,
    pub cleanup: bool,
    pub normalization: bool,
}

#[derive(Debug, Clone)]
pub struct TransformContext {
    pub now_local_iso: String,
    pub now_utc_iso: String,
    pub today_iso: String,
    pub yesterday_iso: String,
    pub tomorrow_iso: String,
    pub dow: String,
    pub dow_abbr: String,
    pub year: i32,
    pub month: u8,
    pub month_name: String,
    pub month_name_abbr: String,
    pub env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct TransformReport {
    pub replacements_applied: usize,
    pub interpolations_applied: usize,
    pub cleanup_changed: bool,
    pub normalization: Option<NormalizationReport>,
    pub warnings: Vec<TransformWarning>,
}
```

Default values:

- `merge_strategy = MergeStrategy::PreferExternal`
- All four Stage 1 stages enabled
- `normalize_target = None`
- `fail_fast = false`
- `context` captured once at pipeline start

## Effective State Resolution

Both replacement and interpolation depend on a shared state map.

Resolution steps:

1. Start from document frontmatter (`self.frontmatter().as_map().clone()`)
2. If `options.state` is present, merge with `options.merge_strategy`
3. Expose merged result as immutable effective state for all Stage 1 steps

Rules:

- Missing keys resolve to `null` in expression evaluation and then to empty string at final string substitution points (unless expression fallback overrides)
- `replace` is read from effective state, not just document frontmatter
- Stage 1 transforms mutate only content by default; frontmatter is not rewritten

## Stage Design Details

### 1) Text Replacement

Selected approach: source-first deterministic scanner (Option 1 from `text-replacement.md`).

Rules:

- `replace` must be a map/dictionary; otherwise skip stage without error
- Keys are literal and case-sensitive
- Empty-string keys are ignored
- Values allowed: scalar JSON values; `null` maps to `""`
- Non-scalar values are ignored in v1
- Single pass, non-recursive replacement
- Overlap resolution: longest key wins, ties lexicographic

Output:

- Updated body content string
- `replacements_applied` count in report

### 2) Frontmatter Interpolation

Selected approach: source-first scanner + expression parser (Option 1 from `interpolation.md`).

Base syntax:

- Variable: `{{foo}}`, `{{user.name}}`, `{{env.HOME}}`, `{{ctx.today}}`
- Fallback: `{{ color | "unknown" }}`
- Truthy ternary: `{{ color ? "known" : "unknown" }}`
- Comparison ternary and helper functions per interpolation design doc

Runtime behavior:

- Capture `ctx.*` once per transform call for deterministic per-run output
- Resolve `env.*` from `options.context.env`, not direct process reads during evaluation
- On unresolved variable with no fallback: empty string

Error policy:

- If expression parse/eval fails and `fail_fast = true`, return `MarkdownError::Transform(...)`
- If expression parse/eval fails and `fail_fast = false`, leave original `{{ ... }}` text and add warning

### 3) Markdown Cleaning

Integration strategy:

- Reuse existing cleanup implementation directly (`cleanup::cleanup_content`)
- Equivalent behavior to current `Markdown::cleanup()`

Notes:

- Existing cleanup behavior includes formatting normalization, table alignment, emphasis/list marker preservation, and code fence handling
- Existing `PREFER_ITALICS` environment behavior remains unchanged

### 4) Normalization

Integration strategy:

- Reuse `normalize::normalize(content, options.normalize_target)`
- Attach returned `NormalizationReport` into `TransformReport`
- Propagate `NormalizationError` as transform failure

Notes:

- With `normalize_target = None`, current behavior is usually no-op re-leveling
- `relevel()` remains a focused API and is not removed

## Internal Module Layout

Recommended additions under `darkmatter/lib/src/markdown/`:

- `transform/mod.rs` - orchestration entry, public types, stage runner
- `transform/types.rs` - `TransformOptions`, `TransformReport`, warnings
- `transform/state.rs` - effective-state merge and lookup helpers
- `transform/replacement.rs` - text replacement engine
- `transform/interpolation/` - lexer/parser/evaluator/context resolution

`mod.rs` updates:

- `pub mod transform;`
- new `Markdown` methods delegating to `transform` module

## Error Model

Extend `MarkdownError` with transform-specific variant:

```rust
#[error("Transform error: {0}")]
Transform(String),
```

Guidelines:

- Non-critical, stage-local issues can be warnings in `TransformReport` when `fail_fast = false`
- Structural failures (for example normalization overflow) are returned as errors
- Frontmatter merge conflicts still use existing `FrontmatterMerge` semantics when applicable

## Determinism and Testability

Key deterministic choices:

- Snapshot `ctx` once at pipeline start
- Snapshot environment once via `TransformContext`
- Deterministic replacement precedence for overlapping keys
- Stable stage order

Test plan:

1. Unit tests per stage engine (replacement and interpolation parser/eval)
2. Integration tests for complete Stage 1 ordering
3. Golden tests for representative markdown documents
4. Error/warning tests for `fail_fast` behavior
5. Existing cleanup and normalization tests remain as regression suite

## Implementation Plan (Iterative)

### Milestone 1: Orchestrator Skeleton

- Add `transform` module and public API (`transform`, `transform_with`, `transform_mut`)
- Wire cleanup + normalization stages only
- Add `TransformOptions`/`TransformReport` skeleton and defaults

### Milestone 2: Text Replacement

- Implement replacement stage using deterministic scanner
- Add rule coverage tests (map type checks, overlap precedence, null/scalar coercion)

### Milestone 3: Interpolation Core

- Implement variable lookup, fallback, and truthy ternary
- Add `ctx.*` and `env.*` context plumbing via `TransformContext`

### Milestone 4: Interpolation Comparisons/Helpers

- Implement comparison operators and helper functions (`length`, `number`, `round`)
- Expand parser/evaluator tests for precedence and type coercion

### Milestone 5: Hardening and Docs

- Add end-to-end fixtures
- Finalize docs and examples
- Reassess module boundaries for Stage 2 readiness

## Open Decisions

1. Interpolation scope policy in v1: apply everywhere or skip code spans/blocks?
2. Should a `clean()` alias be added for discoverability alongside existing `cleanup()`?
3. Should Stage 1 ever mutate frontmatter, or remain content-only through Stage 2?

## Summary

This design introduces a single Stage 1 orchestration API while preserving existing cleanup and normalization behavior, and adds clear integration points for the two missing transforms (text replacement and interpolation). It favors deterministic, source-first transforms now, with a migration path toward parser/AST-aware stages later if needed.

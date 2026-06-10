---
phases: 4
created: 2026-06-10
start_phase: 1
agent: open_code/kimi-for-coding/k2p6
yolo: true
---

# StatusBlock Hint Formatting — Execution Plan

> Derived from: [Functional Specification](spec.md)

## Phase 1: Analysis and Preparation

- [ ] Read and map the current `StatusBlock` implementation in `biscuit-terminal/lib/src/components/status_block.rs`
- [ ] Identify the existing render-tree projection path and the bespoke terminal fallback path
- [ ] Locate existing `StatusBlock` tests in `biscuit-terminal` and understand current test coverage
- [ ] Verify which render targets (Terminal, Markdown, Browser) are exercised by existing tests
- [ ] Confirm the `renderable` style model supports italic emphasis via `TextEmphasis { italic: true, .. }` or equivalent
- [ ] Review Claudine and Darkmatter usage of `StatusBlock` to identify representative test cases for regression

## Phase 2: Core Implementation

- [ ] Update `StatusBlock` render-tree projection to insert a leading blank paragraph inside the body `BlockQuote`
- [ ] Update `StatusBlock` render-tree projection to place a non-blank hint inside the same body `BlockQuote`, separated by a blank paragraph, styled with italic emphasis
- [ ] Ensure blank hints are fully omitted (no separator, no hint node)
- [ ] Preserve existing structural CSS classes: `status-block`, `status-block--{severity}`, `status-block__header`, `status-block__body`, `status-block__hint`
- [ ] Preserve the default-border mapping to a typed thick left border; do not encode custom terminal border prefixes into Markdown or Browser output
- [ ] Ensure the bespoke terminal fallback for `StatusBlock::border()` and Prose-rich headers mirrors the same body/hint layout contract (leading blank line, body in block quote, non-blank hint in block quote, blank separator, italic hint)
- [ ] Verify `StatusState::Failure` behaves identically to `StatusState::Error` during implementation

## Phase 3: Component-Level Testing

- [ ] Add test: every current `StatusState` renders a body with the leading blank line inside the block quote
- [ ] Add test: `StatusState::Failure` renders identically to `StatusState::Error`
- [ ] Add test: body plus non-blank hint renders the hint inside the block quote for Terminal output
- [ ] Add test: body plus non-blank hint renders the hint inside the block quote for Markdown output
- [ ] Add test: body plus non-blank hint renders the hint inside the block quote for Browser output
- [ ] Add test: body plus blank hint omits the separator and hint entirely
- [ ] Add test: hint-only output remains outside the block quote (preserves existing behavior)
- [ ] Add test: multiple body items keep blank-line separation inside a continuous block quote
- [ ] Add test: default-border terminal path uses the shared render-tree projection
- [ ] Add test: custom-border terminal path mirrors the new body/hint layout while honoring the custom prefix
- [ ] Add test: Markdown output does not leak a custom terminal border prefix
- [ ] Add test: Browser output preserves `status-block__hint` class and italic styling for a hint inside a body block quote

## Phase 4: Integration and Regression Validation

- [ ] Run the full `biscuit-terminal` test suite and confirm no regressions
- [ ] Add or update at least one Claudine-facing regression test or snapshot that exercises a composition error with both body and hint, asserting the user-visible contract
- [ ] If a Darkmatter diagnostic uses a custom or unusual `StatusBlock` configuration, add a targeted Darkmatter regression test; otherwise confirm Darkmatter benefits through existing `StatusBlock` usage
- [ ] Perform a visual/manual sanity check of Terminal, Markdown, and Browser output for a representative `StatusBlock` with body + hint
- [ ] Update any affected snapshots or golden files

## Validation Checkpoints

| Checkpoint | Phase | How to verify |
|---|---|---|
| Leading blank paragraph exists in body block quote | 3 | Assert render tree contains empty `Paragraph` as first child of `BlockQuote` |
| Hint inside block quote when body present | 3 | Assert hint `Paragraph` is a descendant of body `BlockQuote` |
| Blank hint omitted | 3 | Assert no hint `Paragraph` and no extra empty `Paragraph` when hint is blank |
| Hint-only unchanged | 3 | Assert hint `Paragraph` is not inside a `BlockQuote` when body is absent |
| Italic hint styling | 3 | Assert `TextEmphasis { italic: true }` or equivalent on hint node |
| Custom-border fallback parity | 3 | Assert terminal output with custom prefix contains hint after separator inside block-quoted region |
| No Markdown leakage of terminal borders | 3 | Assert Markdown output does not contain custom border prefix strings |
| Claudine regression test passes | 4 | Run Claudine test suite; verify snapshot |
| No test regressions | 4 | Full `biscuit-terminal` test suite passes |

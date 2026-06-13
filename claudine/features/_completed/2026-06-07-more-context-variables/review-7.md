---
ready: false
agent: codex
model: ""
---

# Review: Claudine Context Catalogs - Iteration 7

## Findings

### High: the unordered-list right margin still has no Level 2 verification

The specification requires every unordered list to reserve a 1-cell right margin
([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-07-more-context-variables/spec.md:310)).
The implementation configures that margin on `UnorderedList`
([context_render.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context_render.rs:137)),
but the real-terminal test only verifies the marker, hanging indentation, surrounding blank lines,
and that the complete frame does not exceed the pane width
([level2_context_capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_context_capture.rs:461)).
`max_visible_width(frame) <= 100` would still pass if a list line occupied all 100 cells and consumed
the required right margin.

This is a user-observable terminal-layout requirement, so an in-process component configuration is
not sufficient under the required test taxonomy. Add a constrained-width Level 2 capture assertion
that identifies list lines and proves their visible width is at most `pane_width - 1`. Apply it to
the expression report and the side-effect constraint list, which are the two report surfaces that
render unordered lists. Level 3 is not applicable.

## Verification Levels

- Catalog parity, overload parity, flag exclusivity, one-time context capture, null-row inclusion,
  no-effect behavior, report wording, and exact row cardinality: Level 1 present.
- Table widths, margins, box glyphs, inline-code styling, required columns, wrapping, list markers,
  hanging indentation, blank-line spacing, and the 53-cell minimum: Level 2 tmux coverage present.
- Unordered-list 1-cell right margin: implementation and Level 1 coverage only; required Level 2
  verification is missing.
- OS keyboard or mouse behavior: out of scope; Level 3 is not required.

## Verification

- Inspected the specification, current working-tree changes, typed Darkmatter catalogs, runtime
  parity tests, renderer, command tests, PTY tests, and tmux Level 2 suite.
- The existing debug binary showed the list margin rendering correctly in manual captures at 50 and
  53 columns, but the automated Level 2 suite does not assert that contract.
- Cargo and repository-prescribed tests could not run because the host has no active Rust toolchain
  (`rustup show active-toolchain` reports `no active toolchain`).
- `git diff --check HEAD` passes.

## Verdict

Not ready for production. The iteration-6 list-spacing defect is resolved, but one explicitly
specified real-terminal rendering requirement remains verified at the wrong level.

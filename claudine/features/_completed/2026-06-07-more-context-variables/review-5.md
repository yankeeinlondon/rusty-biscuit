---
ready: false
agent: codex
model: ""
---

# Review: Claudine Context Catalogs - Iteration 5

## Findings

### High: `--values` is not verified at the documented 53-cell minimum in a real terminal

The revised specification promises that **every report** preserves all required columns, content,
and the width contract at 53 cells
([spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-07-more-context-variables/spec.md:269),
[spec.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/features/2026-06-07-more-context-variables/spec.md:437)).
The Level 2 default and side-effect tests use 53 cells, but the values test still uses 60 and calls
that its live-data floor
([level2_context_capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_context_capture.rs:607)).

This is the exact class of user-observable rendering requirement that needs Level 2 verification.
The Level 1 command matrix at 53 cannot prove that a real terminal renders widths, borders,
wrapping, and all three columns correctly. Change the Level 2 values capture to 53 cells, or revise
the documented minimum to the actual per-report floor and test that contract.

### Medium: catalog command tests do not enforce “exactly one row”

The default and values tests are documented as checking exactly one row per descriptor, but each
only uses `stdout.contains(property)`
([context_command.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/context_command.rs:368),
[context_command.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/context_command.rs:392)).
The expression and capability checks use the same presence-only pattern. Duplicate rows would pass,
despite the acceptance criterion requiring exactly one context row per descriptor.

Parse rendered data rows or count unwrapped canonical identifiers/signatures at a width where they
remain intact. Assert equality with the descriptor sequence, including no missing, extra, or
duplicate entries.

### Medium: the public CLI reference omits the new minimum-width behavior

The renderer now defines 53 cells as the minimum supported width and allows the shared table
diagnostic below it
([context_render.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context_render.rs:15)).
The `claudine context` CLI reference documents the reports and columns but does not disclose that
floor or the below-floor behavior
([cli-reference.md](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/.claude/skills/claudine/cli-reference.md:440)).

This is public CLI behavior introduced to resolve the prior narrow-terminal finding. Document the
53-cell minimum and what users should expect below it.

## Verification Levels

- Catalog parity, flag exclusivity, wording, one-time capture, null-row inclusion, and no-probe
  behavior: Level 1 present.
- Width caps, margins, glyphs, styling, list rendering, and constrained wrapping: Level 2 present
  for all reports across broad width regimes.
- `--values` preserving `Property`, `Type`, and `Value` at the specified 53-cell floor: strongest
  verification is Level 1; required Level 2 coverage is missing.
- OS keyboard or mouse behavior: out of scope; Level 3 is not required.

## Verification

- Inspected the specification, current implementation, typed Darkmatter catalogs, parity tests,
  command tests, Level 2 tmux captures, and public CLI documentation.
- The existing debug binary rendered all four reports without a planner diagnostic at
  `COLUMNS=53`, but it is not a substitute for the missing automated Level 2 assertion.
- Cargo tests and repository checks could not run because this host has no configured Rust
  toolchain (`rustup show active-toolchain` reports no active toolchain).
- `git diff --check` passes.

## Verdict

Not ready for production. The prior column-dropping defect is resolved, but the revised minimum
width remains unverified at the required level for the live values report.

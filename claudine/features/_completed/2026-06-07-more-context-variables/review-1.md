---
ready: false
agent: codex
model: ""
---

# Review: Claudine Context Catalogs - Iteration 1

## Findings

### High: the context catalog omits three runtime-exposed variables

Darkmatter still inserts the documented backward-compatible `utc`, `dow`, and `dow_abbr` keys into every date/time capture ([capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/context/capture.rs:808)). The new descriptor catalog excludes them from `all_context_variable_keys()` ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/context/catalog.rs:814)), and the command tests were changed to explicitly accept their disappearance ([context_command.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/context_command.rs:84)). Consequently, both `claudine context` reports omit valid `ctx.*` variables even though Darkmatter still produces and documents them.

This violates the requirements to display every runtime context variable and to produce exactly one row per runtime entry. Add descriptors for the aliases, or remove the aliases from the runtime and public contract as a separate compatibility decision. The catalog parity test must include them while they remain runtime-visible.

Verification present: Level 1 command tests now assert only canonical keys. There is no test that compares report rows with the actual captured key set.

### High: the catalog parity tests do not use the runtime implementations as their other side

Each supposed runtime enumerator is another hand-maintained list beside the descriptor list: context keys are repeated in `keys_for_group()` ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/context/catalog.rs:814)), expression names in `all_dispatchable_expression_names()` ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/expression/catalog.rs:356)), and effect signatures in `all_dispatchable_effect_signatures()` ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/effects/catalog.rs:143)). None is derived from capture insertion, expression dispatch, or `EffectEngine` verbs. A developer can add a real runtime entry and forget both catalog lists while every parity test remains green. The missing aliases already demonstrate this failure mode.

This does not satisfy the typed single-source-of-truth or drift-test acceptance criteria. Runtime registration and metadata should come from one declaration, or the tests must exercise the actual runtime surfaces and derive their accepted names/arities from those results.

### High: the required real-terminal rendering behavior has only Level 1 PTY coverage

The file named `level2_context_pty.rs` uses `expectrl` to spawn the binary under a manufactured PTY ([level2_context_pty.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_context_pty.rs:60)). Under the review taxonomy, that is Level 1, even though the tests are named `level2_*` and gated with `Level::L2`. It does not run in WezTerm, Kitty, or tmux and does not capture a real terminal pane.

The spec makes user-visible claims about a 140-cell rendered limit, margins, wrapping, inverse SGR styling, box glyphs, and hanging indentation. Their strongest tests either inspect helper-produced strings in process or inspect PTY bytes. Add genuine Level 2 coverage that captures all four reports in a real terminal at narrow, 140-column, and wider-than-140 sizes, and verifies visible widths, margins, wrapped content, inverse inline code, list indentation, and the 40-column signature cap. Level 3 is not applicable because this feature has no keyboard or mouse behavior.

### High: narrow context reports deliberately remove the required `Type` column

Both context report builders mark `Type` as droppable ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:197), [context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:258)). A new unit test explicitly requires the column to disappear at 78 columns ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:781)). The specification requires the default and values reports to retain `Property`, `Type`, and the final column, relying on wrapping rather than a Claudine-specific alternate narrow layout.

Preserve all three columns and constrain/wrap their content. This user-observable narrow-terminal behavior also needs the genuine Level 2 coverage described above.

### Medium: styled inline code is not consistently rendered through `Prose`

The expression mode table constructs the header as the literal string `` `||` meaning `` ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:520)). `TableColumn` does not run header text through the new inline-code helper, so styled output retains the backticks instead of rendering `||` inversely. Several descriptor-description cells similarly pass `render_inline_code()` markup directly into plain `TableCellContent::Text` rather than rendering it through `Prose` first ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:212), [context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:628)). Current descriptors happen not to contain backticks, but the public metadata API permits them and the renderer contract explicitly covers table cells.

Use one table-cell/header path that renders inline-code-aware `Prose`, and add command-level styled and plain assertions rather than testing only the helper.

### Medium: required capture-count and no-side-effect command tests are assertions by inspection

`render_values_report()` currently calls `ComposeContext::capture()` once ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:226)), but no test measures capture invocations. Likewise, the side-effect command tests check wording but do not prove that no engine construction, filesystem mutation, network request, or policy probe occurred. These are explicit required command tests and performance/safety contracts. Introduce injectable capture/effect boundaries or test instrumentation so regressions fail automatically.

## Verification Levels

- Catalog completeness, flag exclusivity, report selection, wording, and one-row-per-descriptor checks: Level 1 present, but the completeness oracle is not authoritative.
- Width, margins, wrapping, glyphs, inline-code styling, list rendering, and narrow-terminal behavior: Level 1 only. Genuine Level 2 is required and absent.
- OS keyboard/mouse behavior: not in scope; Level 3 is not required.

## Verification

- `git diff --check` passed.
- `sniff repo --plain` identified the `rusty-biscuit` repository.
- Focused Cargo tests and an executable CLI capture could not run because this host has no installed/default rustup toolchain (`rustup toolchain list` reports `no installed toolchains`). No host configuration was changed.

## Verdict

Not ready for production. The implementation omits runtime variables and does not yet establish the required single source of truth. Its terminal rendering contract also lacks the mandated Level 2 verification, and the narrow layout contradicts the specified columns.

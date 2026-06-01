---
ready: false
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Verdict

Not ready for production. The main non-loop `compose`, `inline-compose`, and `sequence` paths have a substantial implementation, but there are still functional gaps around run-scoped invalid optional values, looped composition paths, and test rigor for the interactive terminal behavior.

## Findings

### High: invalid optional values supplied through CLI setters are not dropped

- Requirement: optional properties that are present but invalid should be dropped from the prompt context for this run, warn, then re-compose and re-validate.
- Implementation: invalid optionals are handled by cloning the source Markdown and removing the key from source frontmatter only. The retry reuses the original `PrepareOptions`, so an invalid optional value from `key=value` or `--set` remains in `set_overrides` and fails validation again.
- Impact: `claudine compose prompt.md topic=ok count=bad` fails even when `count` is optional. This violates the spec's run-scoped override behavior and makes CLI-provided optional values stricter than file-authored optional values.
- Evidence: [schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/lib/src/composition/schema_validation.rs:190) retries with `options.clone()`, while [schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/lib/src/composition/schema_validation.rs:312) removes only from `source.markdown.frontmatter_mut()`.
- Reproduction: I ran the compiled CLI with a required `topic`, optional numeric `count`, and setters `topic=ok count=bad`; it exited `1` with Darkmatter schema validation for `/count` instead of dropping `count`.
- Fix direction: drop invalid optional keys from the effective override map as well as source frontmatter before retrying. Add Level 1 tests for invalid optionals from frontmatter, shorthand setters, and `--set`.

### High: looped `compose` and `inline-compose` bypass the schema-aware prepare wrapper

- Requirement: schema support integrates into Claudine's `compose`, `inline-compose`, and `sequence` workflows, validating effective frontmatter before provider launch and applying the Claudine-specific typed errors / optional-drop behavior.
- Implementation: the non-loop single execution path uses `prepare_with_interactive_collection`, but the loop execution closures still call `composition::prepare_direct` and `composition::prepare_inline` directly.
- Impact: prompts that use Claudine loop frontmatter do not get the feature's typed `SchemaLoad`, `SchemaValidation`, `MissingProperties`, `UnsupportedInteractiveSchema`, interactive collection, or optional-drop behavior. Schema failures surface as generic Darkmatter compose failures, and optional invalid values are not handled per spec.
- Evidence: [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:465) and [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:801).
- Fix direction: use the same schema-aware preparation entry points inside loop iteration preparation, with a clear policy for whether interactive collection is allowed before the loop starts or must fail as `MissingProperties`.

### High: interactive schema UX is not verified at the required level

- Requirement: missing required values prompt only when `prompt_for_missing` is enabled, stdin and stderr are TTYs, and `--silent` is off; the TUI must map schema types to widgets, support numeric parse-and-retry, render the schema status report, and avoid opening a TUI in non-TTY mode.
- Verification present: unit tests cover option booleans, value parsing, label formatting, and unsupported-shape branching. CLI tests cover non-interactive missing-property failure, but there are no PTY tests that spawn the binary and feed manufactured input bytes through the TUI.
- Required level: Level 1 PTY coverage is the minimum for the interactive prompt workflow because the user-observable behavior is terminal input/output interaction. If the status report's exact glyphs, SGR colors, or wrapping are contractual, that rendering needs Level 2 real-terminal capture.
- Impact: regressions in stdin/stderr TTY gating, widget operation, cancellation behavior, numeric retry display, or status rendering can ship without test failure.
- Evidence: direct compose schema tests cover only non-interactive process behavior and completions in [compose_schema_cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/tests/compose_schema_cli.rs:1); interactive helper tests are in-process only in [schema_interactive.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/schema_interactive.rs:596).
- Fix direction: add Level 1 PTY tests for missing string, enum, boolean, and numeric values; numeric invalid-then-valid retry; `--silent`; stdin-only/stderr-only non-TTY denial where possible. Add Level 2 capture for the status report if exact styled output remains part of the acceptance contract.

### Medium: sequence interactive unsupported shapes lose the specific unsupported-schema error

- Requirement: unsupported missing required shapes should report that the property cannot be collected interactively from the available schema metadata.
- Implementation: direct `compose` promotes unsupported missing shapes to `UnsupportedInteractiveSchema`, but sequence calls `collect_missing_values` and maps any error back to aggregated `SequenceMissingProperties`.
- Impact: interactive sequence users with a required `object`, `any`, raw JSON Schema, or property-level union do not get the clearer unsupported-shape error that the direct path provides.
- Evidence: direct promotion exists in [schema_interactive.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/schema_interactive.rs:218); sequence collapses collection errors at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/wrap/sequence.rs:713).
- Fix direction: detect unsupported missing properties during sequence aggregation and include that note in the sequence error, or add a sequence-specific unsupported-interactive variant.

### Medium: completion coverage misses file completions and several commands

- Requirement: schema-aware completion applies to `compose`, `inline-compose`, and `sequence`; enum values, file values filtered by `match(...)`, required-before-optional ordering, and already-supplied filtering must work.
- Verification present: Level 1 completion tests cover `compose` property ordering, enum values, and supplied-property filtering.
- Gaps: no integration test proves `inline-compose` or `sequence` route through the schema completer, and no integration test covers `file(match(...))` completion through the CLI completion engine.
- Evidence: [compose_schema_cli.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/tests/compose_schema_cli.rs:185) covers only `compose`; file matching has unit coverage in the completion module but not CLI completion integration.
- Fix direction: add Level 1 completion integration tests for `inline-compose`, `sequence`, and file match candidates.

## Test Rigor Classification

- Direct `compose` non-interactive missing required: Level 1 process coverage exists.
- Direct `compose` invalid required: Level 1 process coverage exists.
- Direct `compose` required setter success: Level 1 process coverage exists.
- `inline-compose` schema validation: library unit coverage exists, but no CLI integration coverage.
- `sequence` non-interactive aggregation and setter success: Level 1 process coverage exists.
- Interactive missing-property prompting: strongest coverage is in-process unit tests; needs Level 1 PTY.
- Status report styled rendering: semantic unit coverage exists; needs Level 2 only if exact glyph/color rendering is a production requirement.
- Shell completion: Level 1 CLI completion coverage exists for part of `compose`; gaps remain for `inline-compose`, `sequence`, and file completions.

## Verification Run

- `cargo test --color=never -p claudine schema_validation` passed.
- `cargo test --color=never -p claudine-cli --test compose_schema_cli` passed.
- Manual CLI reproduction confirmed the invalid optional setter bug described above.

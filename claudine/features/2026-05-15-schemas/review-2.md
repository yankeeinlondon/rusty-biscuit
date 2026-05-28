---
ready: false
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Verdict

Not ready for production. The implementation has moved in the right direction since review 1, especially for invalid optional CLI setters, but the main CLI execution order still lets Darkmatter's raw schema validation fail during shell-command discovery before Claudine's schema-aware prepare wrapper can return `MissingProperties`, run Interactive Mode, or aggregate sequence failures.

## Findings

### High: shell preflight bypasses the schema-aware missing-property path

- Requirement: `compose`, `inline-compose`, and `sequence` must validate effective frontmatter through Claudine's schema wrapper, returning typed `MissingProperties` in non-interactive runs and prompting in Interactive Mode when allowed.
- Implementation: shell-command discovery runs before `prepare_with_interactive_collection` / `prepare_direct_with_schema`. `resolve_shell_approvals` calls Darkmatter's `collect_shell_commands`, which performs schema validation and returns a raw `MarkdownError::SchemaValidationFailed` before Claudine's wrapper gets control.
- Evidence: direct compose preflight runs at [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:437), while schema-aware prepare does not run until [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:579). Sequence does the same per step at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/wrap/sequence.rs:819), before the `prepare_direct_with_schema` match at [sequence.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/wrap/sequence.rs:835). The preflight source is [preflight.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/lib/src/composition/preflight.rs:55).
- Reproduction: running `claudine compose --goose plan.md` with `$schema: { topic: string(required) }` and no `topic` exits with a raw `MarkdownError: schema validation failed`, not Claudine's `CompositionError: missing properties`. Running `claudine sequence --goose seq.md` with two steps has the same raw Markdown error and does not emit the step-aggregated `SequenceMissingProperties` report.
- Impact: non-interactive errors do not match the spec, Interactive Mode cannot be reached for missing required values in the normal CLI path, and sequence users lose the promised "fix every step in one edit" aggregation.
- Fix direction: move schema-aware prepare / missing-value collection before shell discovery, or add a shell-discovery mode that does not run schema validation. Avoid translating this only at the preflight boundary unless the sequence path can still aggregate all step failures.

### High: interactive schema behavior is still not verified at the required level

- Requirement: missing required values prompt only when `prompt_for_missing` is enabled, stdin and stderr are TTYs, and `--silent` is off; widgets must collect strings, enums, booleans, and numbers with retry.
- Verification present: in-process unit tests cover option booleans and helper parsing; process tests cover non-interactive exits only loosely.
- Required level: Level 1 PTY tests are the minimum for the prompt workflow because this is terminal input/output behavior. Level 2 is needed if the styled status report glyphs/colors/wrapping are part of acceptance.
- Impact: the current suite did not catch the preflight blocker above because the CLI tests only asserted that stderr contained words like `missing` and the property name. They passed even when the error surface was Darkmatter's raw schema error rather than Claudine's typed report.
- Fix direction: add PTY tests for string, enum, boolean, numeric invalid-then-valid retry, cancellation, `--silent`, and stdin/stderr TTY gating. Tighten existing process tests to assert `CompositionError` / `missing properties` and, for sequence, per-step aggregation.

### Medium: `inline-compose` can rewrite prompt-property errors before the prompt check runs

- Requirement: `inline-compose` keeps its existing `prompt` validation behavior; missing `prompt` returns `PromptPropertyMissing`, non-string `prompt` returns `PromptPropertyWrongType`, and schema validation runs after those checks.
- Implementation: `drop_invalid_optionals` runs before the inline prompt-property check at [compose.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/cli/src/commands/compose.rs:713). That helper removes any invalid optional property from source frontmatter at [schema_validation.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine-schemas/claudine/lib/src/composition/schema_validation.rs:835), including `prompt`.
- Reproduction: with frontmatter `$schema: { prompt: string }` and `prompt: 123`, `inline-compose` reports "frontmatter is missing a `prompt` property" instead of "frontmatter `prompt` must be a string, got number".
- Impact: the schema pre-scrub changes an inline-specific contract and can make users fix the wrong problem.
- Fix direction: perform the inline `prompt` extraction/type check before optional schema scrubbing, or teach the scrubber not to remove inline-reserved control properties before their command-specific validation runs.

### Medium: schema completion coverage is still incomplete

- Requirement: schema-aware completion applies to `compose`, `inline-compose`, and `sequence`; enum and `file(match(...))` values use Darkmatter completion metadata.
- Verification present: CLI integration tests cover `compose` property names, enum values, and supplied-name filtering. Unit tests cover file matching inside the schema completion module.
- Gap: there is still no CLI integration test proving `inline-compose` and `sequence` route through the schema-aware completer, and no CLI integration test for `file(match(...))` candidates.
- Fix direction: add Level 1 completion integration tests for `inline-compose`, `sequence`, and `file(match('*.ext'))` value completion.

## Test Rigor Classification

- Direct `compose` missing required: strongest current CLI coverage is Level 1 process, but it asserts too loosely and currently accepts the wrong raw Darkmatter error surface.
- `sequence` missing required aggregation: strongest current CLI coverage is Level 1 process, but it does not verify per-step aggregation and currently accepts the wrong raw Darkmatter error surface.
- Invalid optional CLI setters: Level 1 unit and process coverage exists and passed in my run.
- Interactive prompting: strongest coverage is in-process unit tests; needs Level 1 PTY before production readiness.
- Status report styled rendering: semantic unit coverage exists; use Level 2 real-terminal capture if exact styled output remains contractual.
- Shell completion: Level 1 CLI coverage exists for part of `compose`; gaps remain for `inline-compose`, `sequence`, and file completions.

## Verification Run

- `cargo test --color=never -p claudine schema_validation` passed.
- `cargo test --color=never -p claudine-cli --test compose_schema_cli` passed.
- `cargo test --color=never -p claudine-cli --test sequence_cli sequence_unsupported_shape_surfaces_typed_error_under_tty_pref` passed, but the assertion is too weak to prove the stated behavior.
- `cargo test --color=never -p claudine-cli --test sequence_cli sequence_aggregates_missing_required_properties_across_steps` passed, but manual reproduction showed the process emitted raw `MarkdownError: schema validation failed` rather than a sequence aggregate.

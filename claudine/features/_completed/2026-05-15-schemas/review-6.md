---
ready: true
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Verdict

Iteration 6 findings have been addressed (see [Resolution](#resolution)).

Original verdict (iteration 6): Not ready for production. Iteration 6 closes the iteration-5 sequence gaps: non-interactive unsupported sequence shapes now return the aggregated sequence error, and the sequence prompt path has PTY coverage for deduplication, pre-launch prompting, step overlays, and setter-aware status reports.

Two gaps remain. The larger one is behavioral: schema validation still does not validate the final post-shell-expanded effective frontmatter required by the spec. The smaller one is user-facing: invalid optional values are silently dropped in normal CLI runs because the "warning" is only a tracing event, and tracing defaults to off.

## Findings

### High: schema validation does not run on post-shell-expanded effective frontmatter

- Requirement: Claudine must validate after Darkmatter composition has produced the effective frontmatter, and validation must occur before provider/model resolution or provider launch.
- Implementation: Darkmatter now runs schema validation after frontmatter interpolation but before frontmatter shell expansion at `darkmatter/lib/src/markdown/compose/mod.rs:528`. The comment explicitly says post-shell values can be revalidated downstream, but `prepare_direct_with_schema` / `prepare_inline_with_schema` only translate compose errors; on a successful compose they never validate `PreparedComposition::effective_frontmatter` again. Claudine's pre-validation also validates a raw frontmatter-plus-overrides instance at `claudine/lib/src/composition/schema_validation.rs:852` and only defers `{{ ... }}` template-bearing invalid values at `claudine/lib/src/composition/schema_validation.rs:861`, not `$(...)` frontmatter shell expressions.
- Impact: a schema-constrained frontmatter value that is invalid before frontmatter shell expansion but valid afterward is rejected too early, while a value that is valid before shell expansion but invalid afterward can pass through unless an earlier stage happens to reject it. That violates the spec's effective-frontmatter validation contract.
- Fix direction: after `prepare_direct` / `prepare_inline` succeeds, validate `prepared.effective_frontmatter` against the resolved schema and apply the same typed error mapping and optional drop rules against the composed frontmatter. Pre-validation can still exist for prompt collection, but it must not be the final authority for values changed by composition.
- Test coverage needed: add Level 1 process tests for frontmatter shell-expanded schema values in direct `compose`, `inline-compose`, and `sequence`: one case where `$(...)` produces a valid enum/string value and one where it produces an invalid final value. These tests should assert no provider launch on final invalid values.

### Medium: invalid optional values are dropped without a visible warning

- Requirement: optional properties that are present but invalid should be dropped "with a warning" before re-compose/re-validate.
- Implementation: the drop paths use `tracing::warn!` at `claudine/lib/src/composition/schema_validation.rs:330`, `claudine/lib/src/composition/schema_validation.rs:361`, `claudine/lib/src/composition/schema_validation.rs:1026`, and `claudine/lib/src/composition/schema_validation.rs:1036`. Normal CLI tracing defaults to `LevelFilter::OFF` at `claudine/cli/src/telemetry.rs:47`, so users do not see these warnings unless they opt into debug/tracing. The CLI test for invalid optional setters only asserts success at `claudine/cli/tests/compose_schema_cli.rs:284`, not that a warning is emitted.
- Impact: users can pass `count=bad`, have Claudine silently remove it from the run, and never learn that the prompt executed without their supplied optional value.
- Fix direction: return structured dropped-optional diagnostics from schema preparation/pre-validation and render them through the CLI's normal stderr surface (`log::warn` / `Status`), respecting `--silent` if that is intended. Strengthen CLI tests to assert the warning text and dropped property name for file-authored and setter-authored invalid optionals.

## Test Rigor Classification

- Direct `compose` missing required, setter-supplied required, invalid required, invalid optional drop success, loop missing, and inline prompt-property precedence: Level 1 process coverage present.
- Direct `compose` Interactive Mode for string, enum, boolean, numeric retry, `--silent`, and templated enum status drift: PTY coverage present in `level2_schema_prompt_pty.rs`; this is sufficient for the prompt-input behavior, though the filename/gate labels it as Level 2.
- `sequence` non-interactive missing aggregation, setter-supplied success, and unsupported non-TTY aggregation: Level 1 process coverage present.
- `sequence` Interactive Mode prompt/dedupe/pre-launch behavior and status report overlay/setter handling: PTY coverage present in `level2_schema_prompt_pty.rs`; this closes the prior high verification gap.
- Schema setter completion: Level 1 `__complete` coverage present for required ordering, supplied-property filtering, enum values, inline-compose, sequence, basename `file(match(...))`, path-qualified globs, and negated path-qualified globs.
- Post-shell-expanded schema validation: no targeted coverage found; this is a high behavioral gap.
- Invalid optional warning visibility: only success is tested; no CLI stderr assertion verifies the required warning.

## Verification

- Source review of the current schema validation, sequence, completion, and PTY test changes.
- Attempted a direct CLI repro for shell-expanded schema values with `cargo run -p claudine-cli --bin claudine`; the invocation exceeded the non-interactive 60-second limit during build/run and was abandoned.

## Resolution

Both findings addressed.

### High: post-shell schema validation

- **Darkmatter** (`darkmatter/lib/src/markdown/compose/schema_validation.rs`): the compose-time validator now filters out problems whose top-level value contains `$(...)`, deferring them to the prepare-time consumer. This unblocks shell-expanded values that satisfy the schema only after composition.
- **Claudine** (`claudine/lib/src/composition/schema_validation.rs`): a new `post_shell_validate` stage runs after `prepare_direct` / `prepare_inline` succeeds. It revalidates `prepared.effective_frontmatter` against the resolved schema and applies the same typed error rules — `SchemaValidation` for newly-invalid required values, `MissingProperties` for newly-missing required values, drop-with-diagnostic for newly-invalid optional values. Pre-validation's `value_needs_composition` now also recognizes `$(...)` so shell-bearing values defer just like template-bearing values.
- **Tests** (`claudine/cli/tests/compose_schema_cli.rs`, `claudine/cli/tests/sequence_cli.rs`, `claudine/lib/src/composition/schema_validation.rs`): Level 1 process tests cover valid post-shell, invalid post-shell required (no provider launch), and dropped post-shell optional for direct compose, inline-compose, and sequence.

### Medium: dropped-optional warning visibility

- **Library** (`claudine/lib/src/composition/error.rs`, `types.rs`, `schema_validation.rs`): a new `DroppedOptional` struct records the property name, source (`Frontmatter` / `Override` / `Composed`), pipeline stage (`PreValidation` / `Composition` / `PostShellExpansion`), and validator reason. `PreparedComposition` carries `dropped_optionals: Vec<DroppedOptional>`; `PreValidatedSchema` carries the same field; `drop_invalid_optionals` returns the drop log alongside the scrubbed source / overrides.
- **CLI** (`claudine/cli/src/commands/schema_interactive.rs`): new helper `emit_dropped_optional_warnings` renders each entry through `log::warn` (yellow, always-on stderr) so users see "warning: dropped optional schema property …" on every run. Called from `compose`, `inline-compose`, and `sequence` after pre-validation and after `prepare_*_with_schema`.
- **Tests** (`claudine/cli/tests/compose_schema_cli.rs`): `compose_invalid_optional_in_file_emits_visible_warning` and `compose_invalid_optional_setter_emits_visible_warning` assert the stderr warning text and the dropped property name.

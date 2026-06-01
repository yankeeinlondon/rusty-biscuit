---
ready: false
agent: codex
model: ""
---

# Review: Schema Support in Claudine

## Verdict

Not ready for production. Iteration 4 closes the prior major gaps: non-interactive compose/inline/sequence paths have Level 1 process coverage, schema setter completions now cover `compose`, `inline-compose`, `sequence`, and basename `file(match(...))`, and the interactive prompt has PTY coverage for string, enum, boolean, number retry, and `--silent`.

Two user-facing gaps remain. One is a functional completion bug for path-qualified Darkmatter file globs. The other is an interactive status-report mismatch: the report is built from raw frontmatter before Claudine installs the provider-derived composition environment, so prompts that execute correctly can still display incorrect schema status before asking for missing values.

## Findings

### Medium: `file(match(...))` completion only matches basenames, not Darkmatter path globs

- Requirement: schema-aware completion for `file` values must use Darkmatter completion metadata, and Darkmatter's `file(match(...))` semantics match the resolved path against glob patterns. The schema docs explicitly show path-qualified patterns such as `src/**/*.rs` and `!src/**/test_*.rs`.
- Implementation: the completion walker computes the relative path, but filters glob patterns against only `path.file_name()` at `claudine/cli/src/completion/schema_completion.rs:263-267`. A schema like `source_code: "file(match('src/**/*.rs', '!src/**/test_*.rs'))"` will not offer `src/lib.rs`, because `src/**/*.rs` is compared to `lib.rs`.
- Evidence: Darkmatter documents path-qualified match globs at `darkmatter/docs/topics/schema-definition.md:151-155`. Claudine's only integration test uses `file(match('*.png'))`, which exercises basename extension filtering but not path-qualified or negated path globs, at `claudine/cli/tests/compose_schema_cli.rs:763-790`.
- Impact: users relying on the documented Darkmatter schema contract get incomplete or empty completion results for common scoped file properties, even though validation accepts those same values.
- Fix direction: match positive and negative globs against the same relative path string that will be emitted as the completion candidate, while preserving basename fuzzy filtering for the active partial if desired. Add Level 1 `__complete` tests for `src/**/*.rs` and `!src/**/test_*.rs`.

### Medium: interactive schema status can report raw template values as invalid

- Requirement: the status report shown before Interactive Mode should reflect the schema status of the composition run. The spec's validation target is the effective frontmatter after run-scoped overrides and composition context are applied.
- Implementation: `compose` calls `pre_validate_with_interactive_collection` immediately after loading the source at `claudine/cli/src/commands/compose.rs:368-388`. The provider-derived `AGENT` environment is not installed until later at `claudine/cli/src/commands/compose.rs:426-446`. When prompting is allowed, the interactive helper renders `build_schema_status_report(source, set_overrides)` at `claudine/cli/src/commands/schema_interactive.rs:225-226`, and that report validates a raw frontmatter map plus CLI overrides at `claudine/lib/src/composition/schema_validation.rs:641-655`.
- Reproduction shape: a prompt with `runtime_agent: "{{ env.AGENT }}"`, `$schema.runtime_agent: "enum(goose; required)"`, and a separate missing required `topic` will enter Interactive Mode for `topic` under `claudine compose --goose`. The command can execute after collection, but the status report is built without the `AGENT=goose` composition context and can mark `runtime_agent` invalid.
- Impact: the user sees a wrong "defined but with the wrong type" schema status immediately before being asked for missing values. That undermines the diagnostic report even though the later preflight/prepare path was fixed to compose with `env_overrides`.
- Fix direction: either build the status report from the same compose context used by preflight/prepare, or make the report composition-tolerant in the same way `pre_validate_schema` is for deferred template-bearing values. Add a PTY test that combines a missing required prompt value with a provider-derived templated enum value and asserts the status transcript does not report the templated property as invalid.

## Test Rigor Classification

- Direct `compose` missing, invalid required, invalid optional drop, and setter-supplied required values: Level 1 process coverage present.
- `inline-compose` prompt-property precedence and missing schema values: Level 1 process coverage present.
- `sequence` preflight aggregation and setter-supplied schema values: Level 1 process coverage present.
- Interactive prompting for string, enum, boolean, numeric retry, and `--silent`: PTY coverage present. Under the review prompt taxonomy this is Level 1 PTY, which is appropriate for prompt input behavior.
- Schema setter completion: Level 1 `__complete` coverage present for required ordering, supplied-property filtering, enum values, `inline-compose`, `sequence`, and basename `file(match('*.png'))`. Path-qualified and negated file glob coverage is missing.
- Status report rendering: semantic unit coverage exists, and PTY coverage exercises prompt flow. The remaining gap is semantic correctness for templated/effective values during Interactive Mode, not terminal color fidelity.

## Verification Run

- `cargo test --color=never -p claudine-cli --test compose_schema_cli -- --nocapture`
- `cargo test --color=never -p claudine-cli --test sequence_cli -- --nocapture`
- `cargo test --color=never -p claudine-cli --test level2_schema_prompt_pty -- --nocapture`
- `cargo test --color=never -p claudine schema_validation -- --nocapture`

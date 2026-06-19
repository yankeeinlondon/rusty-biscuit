---
ready: false
agent: codex
model: ""
---

# Review: `claudine context`

## Verdict

Not ready for production. The command is wired and has basic smoke tests, but there are requirement gaps around how the report is sourced and rendered. The current tests are Level 1 process tests and mostly assert that output is non-empty, so they do not prove the required report content.

## Findings

### High: `claudine context` depends on source-tree docs at runtime

- Requirement: `claudine context` must provide a full overview of Darkmatter context variables and `--expressions` must report expression-engine operations/functions.
- Implementation: `context_variables_path()` and `expressions_path()` walk upward from the current working directory looking for `darkmatter/docs/topics/*.md`, then silently fall back to a relative path. `parse_context_variables()` / `parse_expressions_doc()` use `read_to_string(...).unwrap_or_default()`.
- Impact: an installed `claudine` binary run outside the monorepo will not have those docs under the user's CWD. Default `claudine context` can produce no context-variable tables at all and still exit successfully; `--expressions` can render only the hard-coded title/intro with the detailed sections omitted.
- Evidence: `claudine/cli/src/commands/context.rs:51`, `claudine/cli/src/commands/context.rs:70`, `claudine/cli/src/commands/context.rs:332`, `claudine/cli/src/commands/context.rs:364`.
- Fix direction: embed the authoritative docs with `include_str!` or expose structured context/expression metadata from Darkmatter. If file loading remains, missing docs must be a visible error, not empty success.
- Verification gap: strongest coverage is Level 1, but only from `repo_root()`; no test runs the installed-style binary from an arbitrary temp directory.

### High: property names are rendered without the required `ctx.` prefix

- Requirement: the "Property" column should show properties like `ctx.today`.
- Implementation: table rows reuse the Darkmatter docs' `Variable` value directly, e.g. `` `today` ``, and `--values` does the same.
- Impact: the report does not show the actual expression/interpolation path users must type. This is a user-facing correctness issue for the core command output.
- Evidence: `claudine/cli/src/commands/context.rs:242`, `claudine/cli/src/commands/context.rs:297`.
- Fix direction: normalize display names to `ctx.<key>` while keeping lookups against the raw key.
- Verification gap: Level 1 tests assert only non-empty stdout, not required column headers or representative values such as `ctx.today`.

### High: test coverage does not verify the specified report content

- Requirement: default output is organized by H3/H4 headings with `Property`, `Type`, `Description`; `--values` replaces `Description` with `Value`; footer messages are emitted; `--expressions` is well structured and includes operations/functions; `--side-effects` prints `not implemented yet`.
- Implementation tests: `claudine/cli/tests/context_command.rs` checks exit success and non-empty stdout for most modes, plus substring checks for footer flags and the side-effects placeholder.
- Impact: regressions can drop most headings, columns, variables, values, and expression sections without failing tests.
- Evidence: `claudine/cli/tests/context_command.rs:9`, `claudine/cli/tests/context_command.rs:22`, `claudine/cli/tests/context_command.rs:35`, `claudine/cli/tests/context_command.rs:64`.
- Fix direction: add Level 1 integration assertions for representative required content in each mode: headers, H3/H4 grouping, at least one known variable from each major section, no `Description` column under `--values`, non-null live values for cheap context keys, and expression sections/functions/operators.
- Verification level: Level 1 is appropriate for this non-interactive reporting command. Level 2 would only be needed if exact styled table rendering, wrapping, or color output becomes a contractual requirement.

### Medium: expression report omits several documented operation areas

- Requirement: `--expressions` should explain where expressions can be used, operations provided, and utility functions in a clear, not overly verbose report.
- Implementation: it renders a hard-coded subset: intro, precedence, truthiness, unary operators, and function subsections. It does not surface comparison operators, arithmetic operators and error behavior, interpolation-vs-condition `||` semantics beyond the intro, variable access, bracket/dot access, null propagation, or date/time behavior.
- Impact: users do not get a complete operations report from `claudine context --expressions`.
- Evidence: `claudine/cli/src/commands/context.rs:574`, `claudine/cli/src/commands/context.rs:588`.
- Fix direction: add concise tables/sections for comparison, arithmetic, logical/fallback semantics, access forms, and null propagation. Keep examples short.
- Verification gap: no Level 1 test asserts any specific expression operation or helper appears.

### Medium: footer does not use the status component requested by the spec

- Requirement: add `StatusInfo::Info` messages to stderr.
- Implementation: `render_footer()` writes styled `Prose` directly through `log::message`.
- Impact: output is less consistent with Claudine's status-line conventions and misses the requested Info status treatment.
- Evidence: `claudine/cli/src/commands/context.rs:315`.
- Fix direction: render with `biscuit_terminal::components::status::Status` and `StatusState::Info`, matching the existing wrapper/compose status usage.
- Verification gap: current tests check only that stderr contains flag names, not that the footer uses the expected status renderer.

## Test Rigor Classification

- `claudine context`: Level 1 process coverage exists, but it is too weak because it only checks non-empty stdout.
- `claudine context --values`: Level 1 process coverage exists, but it does not verify the `Value` column or live values.
- `claudine context --expressions`: Level 1 process coverage exists, but it does not verify operations/functions content.
- `claudine context --side-effects`: Level 1 process coverage verifies the placeholder text.
- Footer messages: Level 1 process coverage verifies flag substrings, but not status rendering.

No Level 2 or Level 3 coverage is required for the current spec because it does not define terminal-keyboard behavior. Level 2 would be useful only for exact styled table/color rendering requirements.

## Suggested Acceptance Criteria

- Run `claudine context` from outside the repository and still get the full context-variable report.
- Default report includes `Property`, `Type`, `Description`, H3/H4 grouping, and properties displayed as `ctx.*`.
- `--values` includes `Property`, `Type`, `Value`, excludes `Description`, and shows live values for representative keys.
- `--expressions` includes concise coverage of expression surfaces, precedence, truthiness, access syntax, comparison/logical/arithmetic operations, null propagation, and all helper groups.
- Tests assert these requirements directly at Level 1.

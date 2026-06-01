---
phases: 5
created: 2026-05-26
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - darkmatter
---

# Execution Plan: Ternary-Conditional Commands in Frontmatter Shell Expansion

This plan outlines the implementation of ternary conditional support in Darkmatter's frontmatter shell expansion (`$(COND ? THEN : ELSE)`). It ensures security by validating all reachable commands against the allowlist before execution.

## Phase 1: Foundations & AST

Define the data structures required to represent the ternary AST and update the existing directive model.

- [x] **Task 1.1: Define AST Types**
    - Create `FrontmatterShellAst` and `Branch` enums in `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`.
    - Implement `Branch::commands()` to return an iterator of executables.
- [x] **Task 1.2: Update FrontmatterShellDirective**
    - Refactor `FrontmatterShellDirective` to hold the new `FrontmatterShellAst`.
    - Maintain backwards compatibility by ensuring existing fields (like `pipeline`) can be derived from the AST or are conditionally present.
- [x] **Task 1.3: Update ShellCommandOrigin**
    - Ensure `ShellCommandOrigin::Frontmatter` remains capable of reporting errors accurately within the new AST structure.

## Phase 2: Parsing Logic

Implement the quote-aware ternary splitter and integrate it into the shell directive parser.

- [x] **Task 2.1: Implement Ternary Splitter**
    - Add `split_top_level_ternary(inner: &str) -> Option<(&str, &str, &str)>` helper.
    - Ensure it is aware of single quotes, double quotes, and parentheses to avoid splitting on `?` or `:` inside them.
- [x] **Task 2.2: Update `parse_shell_value`**
    - Modify the parser to attempt ternary splitting first.
    - If a ternary is detected, parse the `then` and `else` branches as either `Branch::Empty` (for `''` or `""`) or `Branch::Pipeline`.
- [x] **Task 2.3: Refactor Validation**
    - Move `validate_no_executable_interpolation` to operate on `Branch::Pipeline` instances rather than the raw string.
    - Ensure the condition position in a ternary is exempt from this check.
- [x] **Task 2.4: Refine Error Messages**
    - Update `ShellExpansionError::ParseDirective` to include branch-specific context (e.g., "in then-branch of ternary").

## Phase 3: Condition Evaluation & Execution

Integrate the expression engine to evaluate the ternary condition and implement the branching execution flow.

- [x] **Task 3.1: Integrate Expression Engine**
    - Import `darkmatter::compose::expression::evaluate`.
    - In the execution path, evaluate `condition_source` against the `FrontmatterSeedState`.
- [x] **Task 3.2: Implement Branch Selection**
    - Use `is_truthy()` on the evaluated result to select the appropriate branch.
- [x] **Task 3.3: Implement Branch Execution**
    - For `Branch::Empty`, return an empty string immediately.
    - For `Branch::Pipeline`, proceed with existing `prepare_directive` and `execute_prepared_directive` logic.

## Phase 4: Validation & Allowlist Integration

Ensure the security invariant: "Every command that the directive could execute must be statically determinable and allowlisted."

- [x] **Task 4.1: Static Command Discovery**
    - Ensure all commands from both `then_branch` and `else_branch` are gathered during the validation phase.
- [x] **Task 4.2: Allowlist Enforcement**
    - Verify that every command in the reachable set (union of both branches) satisfies the allowlist policy *before* any branch is executed.
- [x] **Task 4.3: Handle Mixed Allowlist States**
    - Verify that if one branch contains an unallowlisted command, the entire directive fails, even if that branch would not have been selected.

## Phase 5: Verification & Testing

Comprehensive testing to ensure correctness, security, and backwards compatibility.

- [x] **Task 5.1: Unit Tests for Ternary Parsing**
    - Test successful parsing of valid ternaries.
    - Test failure cases (missing `:`, multiple `?`, invalid branch content).
    - Test quote/parentheses escaping for `?` and `:`.
- [x] **Task 5.2: Unit Tests for Execution**
    - Test `true` condition selecting `then`.
    - Test `false` condition selecting `else`.
    - Test empty string literals producing no shell invocation.
- [x] **Task 5.3: Security & Allowlist Tests**
    - Verify rejection of interpolated executables in either branch.
    - Verify rejection of off-allowlist commands in either branch.
- [x] **Task 5.4: Integration Test**
    - Implement the worked example from the spec: `spec_file: "$({{has_spec}} ? basename '{{spec}}' : '')"`.
- [x] **Task 5.5: Final Plan Verification**
    - Run existing tests to ensure no regressions in standard shell expansion.

## Review 3 Follow-up

Findings from `review-3.md` addressed in
`darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`,
`darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs`, and
`darkmatter/features/2026-05-26-ternary-conditional-commands/spec.md`:

- **Finding 1 (high)**: ternary condition evaluation now routes through
  `expression::parse_condition` instead of `parse`, so the spec's full
  condition grammar (`&&`, `||`, `!`, comparisons, expression-level
  ternaries) is accepted. Level 1 execution tests added for each form.
- **Finding 2 (high)**: shell-command discovery now flattens ternary
  branches. The new `directive_reachable_pipelines` helper exposes every
  pipeline a directive could execute (one for `Pipeline` AST, one per
  non-`Empty` branch for `Ternary` AST) after running per-branch
  interpolation. `scan_one_frontmatter` iterates that set and emits one
  entry per reachable action. Level 1 discovery tests added covering
  both-branches-emit, empty-branch-emits-nothing, branch-chain-per-action,
  interpolated-arg-resolution, and executable-interpolation rejection.
- **Finding 3 (medium)**: the spec's Decision #3 now documents the
  whitespace-padded separator contract explicitly; a parser test locks in
  that `$(flag? echo yes : '')` is treated as a plain pipeline (not a
  ternary) so the contract is enforced.
- **Side fix**: `find_unquoted_closing_paren` now tracks paren depth, so
  parenthesized condition sub-expressions like `$((a ? b : c) ? then : else)`
  no longer have their outer `$(...)` closed by an inner `)`. This aligns
  the inner-extraction with the splitter's existing top-level view.

## Review 1 Follow-up

Findings from `review-1.md` addressed in `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`:

- **Finding 1 (high)**: branch boundaries now anchor to the original
  (pre-interpolation) snapshot. `Branch::Pipeline` carries `original_text`
  and is interpolated per-branch at execute time, so condition interpolation
  cannot shift the then/else split.
- **Finding 2 (high)**: ternary condition evaluation now interpolates the
  condition source against the seed state first, then parses the result
  as a template expression. A frontmatter value rendered as the string
  `"false"` is recognized as a boolean and selects the else-branch.
- **Finding 3 (medium)**: a top-level `?` or `:` in either branch is
  rejected at parse time with a "nested ternaries are not supported"
  error.
- **Finding 4 (medium)**: added compose-level integration tests covering
  the motivating workflow (true and false branches) and the stringified
  `"false"` interpolation path through the full compose pipeline.

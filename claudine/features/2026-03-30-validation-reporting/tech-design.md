# Validation Reporting Tech Design

This document turns the validation-reporting spec into an implementation-ready design for Claudine's existing harness, wrapper, and composition pipeline.

Primary inputs:

- `claudine/features/2026-03-28-validation-reporting.md/spec.md`
- `claudine/features/2026-03-26-validations/tech-design.md`
- current harness implementation in `claudine/lib/src/harness/`
- current wrapper orchestration in `claudine/cli/src/commands/wrap/`
- current composition preparation in `claudine/lib/src/composition/prepare.rs`
- the `Status` component in `biscuit-terminal/lib/src/components/status.rs`

The core design decision is to treat this as a reporting and preflight feature layered on top of the existing harness model, not as a second validation engine. Validation semantics stay where they are today. What changes is:

1. how preflight status is emitted
2. when shell safety is checked
3. how failures and handler engagement are surfaced

## Summary

Claudine already has a working harness:

1. `parse_harness_plan_with_shell(...)` parses `pre_checks`, `post_checks`, `timeout`, and handlers
2. `evaluate_pre_checks(...)` and `evaluate_post_checks(...)` execute validations
3. `run_harness_loop(...)` retries, resumes, redirects, and deviates through the existing handler model
4. Darkmatter already enforces shell policy for `::shell` directives during composition

The current UX gap is that reporting is split across raw `Prose` lines, generic wrapper warnings, and hard failures that only become visible after the harness has already committed to a path.

This feature adds a first-class validation reporting layer with two responsibilities:

1. render all validation-related feedback through `biscuit-terminal::Status` using the circular theme
2. perform a true preflight shell audit for harness-declared runtime commands and the current source page before the provider launches

The intended result is:

- non-interactive compose flows show a clear source-file status first
- users see how many pre/post checks were discovered before execution starts
- each check result is rendered as a `Status`
- shell policy failures fail early with explicit audit output
- handler engagement is visible as a deliberate recovery transition rather than a silent loop branch

## Goals

1. Use `Status::from_prose(...)` with `StatusTheme::Circular` for all validation-related reporting in non-interactive composition flows.
2. Emit the spec's source-file existence messages before composition or provider launch.
3. Emit explicit discovery summaries for populated `pre_checks` and `post_checks`.
4. Replace ad-hoc per-check `Prose` rendering with structured `Status` output.
5. Add a shell audit pass that checks all harness runtime commands and the current source page before provider launch.
6. Fail fast when shell policy guarantees the run will fail.
7. Surface handler engagement with a dedicated status line when recovery begins.
8. Preserve existing validation semantics, handler precedence, and failure classification.

## Non-Goals

1. Adding new validation types.
2. Changing handler precedence or handler action semantics.
3. Redesigning Darkmatter's shell approval or blacklist model.
4. Changing interactive-session UX.
5. Replacing normal stream summaries with `Status` output. This feature is only about validation and handler reporting.

## Scope Boundary

This reporting layer applies to Claudine's non-interactive, Markdown-backed execution paths:

1. `claudine compose <file>`
2. `claudine inline-compose <file>`
3. wrapper passthrough runs that resolve to a Markdown file and activate the harness

The feature is intentionally not a general terminal-output refactor for every wrapper path.

## Current Baseline

Today the relevant behavior is split across three places:

1. `claudine/lib/src/harness/validate.rs`
   - evaluates checks
   - renders each result immediately as `Prose`
   - prints directly from the validation layer
2. `claudine/cli/src/commands/wrap/mod.rs`
   - runs the harness loop
   - resolves handlers
   - emits generic warnings/errors when attempts fail
3. `claudine/lib/src/composition/prepare.rs`
   - composes Markdown via Darkmatter
   - executes `::shell` directives as part of composition
   - does not perform a Claudine-owned shell audit before compose starts

This creates four concrete problems:

1. reporting is not consistently status-based
2. validation discovery counts are not surfaced
3. shell failures in the source page are only discovered inside composition execution, not by a dedicated preflight
4. handler recovery is operationally correct but visually opaque

## Spec Clarifications

The spec is short and leaves a few important behaviors implicit. This design resolves them explicitly.

### 1. Reporting goes to stderr and respects existing check visibility

Validation reporting is human-facing status output. It should follow the existing wrapper rule used by checks and summaries:

- emit on stderr
- honor the existing `show_checks` / `silent` behavior
- never pollute stdout, which is reserved for provider output and pipeable content

### 2. `StatusState` mapping is fixed

This feature needs deterministic state mapping:

- source file found: `Success`
- source file missing: `Failure`
- phase discovery headers: `Info`
- shell audit header: `Info`
- individual shell audit item approved: `Success`
- individual shell audit item denied/blacklisted/unresolvable: `Failure`
- individual validation pass: `Success`
- individual validation fail: `Failure`
- handler engagement banner: `Warning`
- terminal unhandled failure banner: `Failure`

`Warning` is the right state for handler engagement because an error has occurred, but the run is not terminal yet.

### 3. Shell audit scope for v1

The spec says to audit shell commands in:

1. `pre_checks`
2. `post_checks`
3. the page we are about to compose

For v1, "the page" means the current source Markdown selected for this attempt, not every recursively transcluded descendant. That keeps the implementation local to Claudine's current source file and avoids turning this feature into a Darkmatter graph-analysis project.

Important follow-through:

- harness-declared commands are fully audited up front
- shell directives in the current source page are audited up front
- if a handler redirects to another file, the next loop iteration reruns the same source-page audit for that file
- transcluded descendant directives remain protected by Darkmatter's existing enforcement during composition

This still satisfies the spec's fail-fast intent for the current page while keeping the change bounded.

### 4. Handler banner is emitted once per failure episode

If a failure produces multiple validation failures, Claudine should emit one handler-engagement banner before trying recovery, not one banner per failed check.

## Target UX

For a normal non-interactive run with checks, the visible order becomes:

1. source-file existence status
2. shell audit header and itemized shell audit statuses
3. `pre_checks` discovery header, if populated
4. individual pre-check statuses
5. provider run
6. inline closure status, where applicable
7. `post_checks` discovery header, if populated
8. individual post-check statuses
9. handler-engagement banner if recovery is triggered
10. terminal failure status if recovery does not resolve the problem

The spec's exact source-file messages should be preserved:

- success: `the file reference was resolved to <blue-500><a href="{absolute_path}">{filepath}</a></blue-500> file on this host`
- failure: `the file reference <blue-500>{ref}</blue-500> found no match on host computer!`

All user-controlled fragments inserted into `Prose` markup must be escaped before interpolation.

## Recommended Module Changes

### `claudine/lib/src/harness/report.rs` (new)

Own all validation-reporting presentation logic.

Recommended responsibilities:

1. render `Status` lines with fixed theme and prose support
2. render source-file existence status
3. render phase discovery headers
4. render individual validation results
5. render shell audit headers and items
6. render handler-engagement and terminal failure banners

This keeps `validate.rs` focused on evaluation instead of printing.

### `claudine/lib/src/harness/audit.rs` (new)

Own shell-command discovery and policy preflight.

Recommended responsibilities:

1. collect shell-bearing runtime commands from a `HarnessPlan`
2. parse `::shell` directives from the current source Markdown text
3. validate each command against the existing shell policy
4. return a structured audit report without executing the commands

### `claudine/lib/src/harness/validate.rs` (refactor)

Stop printing directly from the validation engine.

Instead, return structured per-rule outcomes so the wrapper can decide when to emit statuses.

### `claudine/cli/src/commands/wrap/mod.rs` (integration)

Insert reporting and shell-audit calls into `run_harness_loop(...)` at deterministic points:

1. source-file status before harness parsing
2. shell audit after plan parse and before pre-check execution
3. phase discovery status before each phase's checks
4. handler banner before applying a recovery action
5. terminal failure banner before returning an error to the command layer

### `claudine/lib/src/composition/prepare.rs` or wrapper source-loading entrypoints

Expose enough source context so reporting can show the original file reference string, not just the resolved path.

The simplest change is to carry both:

- original file reference
- resolved absolute source path

through the harness prompt state.

## Data Model Additions

### Validation execution outcomes

Add a structured result model for evaluation output:

```rust
pub struct ValidationCheckOutcome {
    pub rule_id: ValidationRuleId,
    pub event: ValidationEvent,
    pub subject_key: Option<String>,
    pub passed: bool,
    pub markup: String,
    pub failure_message: Option<String>,
}

pub struct ValidationPhaseReport {
    pub phase: FailurePhase,
    pub outcomes: Vec<ValidationCheckOutcome>,
}
```

`markup` is the prose-ready message body, not the rendered ANSI string. Rendering belongs in `report.rs`.

### Shell audit model

Add a dedicated audit report:

```rust
pub enum AuditedCommandSource {
    PreCheck(ValidationRuleId),
    PostCheck(ValidationRuleId),
    ProgrammaticHandle,
    DeclarativeHandler { event: FailureEvent, subject_key: Option<String> },
    ComposeSourceLine { line: usize },
}

pub struct AuditedCommand {
    pub source: AuditedCommandSource,
    pub raw: String,
    pub executable: String,
    pub args: Vec<String>,
}

pub struct ShellAuditOutcome {
    pub command: AuditedCommand,
    pub passed: bool,
    pub message: String,
}

pub struct ShellAuditReport {
    pub outcomes: Vec<ShellAuditOutcome>,
}
```

The message should already be prose-ready and human-readable.

## Shell Audit Design

### Commands that must be audited

The audit pass should inspect these command sources:

1. `ValidationKind::ShellCommand`
2. programmatic `handle`
3. declarative `deviate` handlers
4. `::shell` directives parsed from the current source page text

`retry`, `resume`, and `redirect` do not add shell commands themselves and do not participate in the audit.

### Validation path reuse

Do not create a second shell-policy implementation.

The audit should reuse the same policy primitives already used by the harness parser and Darkmatter shell expansion:

1. tokenize / normalize commands the same way
2. use the same policy root resolution
3. use the same whitelist / blacklist files
4. use the same approval behavior contract

For harness runtime commands, the audit can reuse the existing `validate_and_approve_command(...)` and `validate_and_approve_command_parts(...)` code paths, but it must collect a report entry instead of only succeeding or failing.

For source-page `::shell` directives, use Darkmatter's public shell-expansion parser and policy helpers rather than trying to re-parse shell syntax manually.

### Failure behavior

If any audit item fails:

1. emit its failure status
2. emit a terminal validation failure banner if no recovery path applies
3. return an error before the provider launches

Shell audit failures should behave like pre-launch harness failures, not like warnings.

### Handler interaction

Shell audit happens before pre-check execution. If the audit fails, Claudine should still allow the existing handler model to react when the failure originated from a harness-declared runtime command.

Practical rule:

- failures from harness-declared commands are converted into failure contexts that can flow through normal handler resolution
- failures from source-page `::shell` directives are terminal in v1 because they are not represented as harness events today

This keeps the initial design implementable without inventing a new handler event taxonomy for composition directives.

## Reporting Design

### Status renderer contract

Every validation-reporting line should be rendered through a single helper equivalent to:

```rust
Status::from_prose(markup)
    .state(state)
    .theme(StatusTheme::Circular)
    .render(term)
```

There should be no direct `Prose::render(...)` calls left in the validation-reporting path after this feature.

### Discovery headers

When `pre_checks` or `post_checks` are populated, emit:

- `Info` status
- exact singular/plural grammar
- no header when the phase has zero checks

Recommended markup shape:

```text
<b>{count}</b> validation <i>pre {check|checks}</i> {was|were} found:
<b>{count}</b> validation <i>post {check|checks}</i> {was|were} found:
```

### Validation result items

Each evaluated rule becomes one status line:

- `Success` for pass
- `Failure` for fail
- description comes from the existing message-template/default-message system

This preserves current author overrides while upgrading the presentation layer.

### Handler engagement

Before Claudine applies the chosen recovery action, emit one `Warning` status using the spec text:

```text
an <red>error</red> was encountered while processing <blue>{file}</blue>, engaging registered handlers.
```

`{file}` should be the current attempt's source path display, prose-escaped.

### Unhandled failure banner

If the failure is unhandled, emit a `Failure` status before returning the command error. The final CLI error still matters for exit code propagation, but the user should already have seen a structured status line.

## Wrapper Integration Plan

Inside `run_harness_loop(...)`, the order should become:

1. emit source-file existence status for `prompt_state.source_path`
2. parse harness plan
3. insert inline writability pre-check, if needed
4. run shell audit and emit its statuses
5. emit `pre_checks` discovery header
6. evaluate pre-checks and emit per-check statuses
7. handle pre-check failure if needed
8. run provider attempt
9. run inline closure, if needed
10. emit `post_checks` discovery header
11. evaluate post-checks and emit per-check statuses
12. handle post-check failure if needed

When a handler is selected at steps 7, 8, 9, or 12:

1. emit handler-engagement banner once
2. emit optional handler `msg`
3. apply the next-attempt plan
4. loop

If a redirect changes the source file, the next iteration naturally re-emits:

1. source-file status for the redirected file
2. shell audit for the redirected file
3. the new phase discovery headers

## API Shape Recommendations

To minimize churn, keep existing top-level function names where possible and add detailed variants underneath:

```rust
pub fn evaluate_pre_checks(...) -> Result<ValidationPhaseReport, HarnessError>
pub fn evaluate_post_checks(...) -> Result<ValidationPhaseReport, HarnessError>
pub fn audit_shell_commands(...) -> Result<ShellAuditReport, HarnessError>
```

`HarnessError::PreCheckFailed` and `HarnessError::PostCheckFailed` can continue carrying `Vec<ValidationFailure>`, but the wrapper should receive the richer phase report before the error is collapsed into a terminal result.

One clean pattern is:

1. `validate.rs` produces `ValidationPhaseReport`
2. helper converts failed outcomes into `Vec<ValidationFailure>`
3. wrapper renders the report, then branches on failures

That keeps evaluation, reporting, and control flow separate.

## Testing Plan

### Unit tests

Add focused unit coverage for:

1. source-file status message rendering
2. pre/post discovery header grammar
3. `StatusState` mapping for each report type
4. shell audit collection from `ValidationKind::ShellCommand`
5. shell audit collection from `handle` and `deviate`
6. shell directive parsing from source-page Markdown
7. evaluation functions returning structured outcomes without printing

### Integration tests

Add wrapper-level tests covering:

1. missing source file reports failure before provider launch
2. populated `pre_checks` emits the discovery header and itemized statuses
3. populated `post_checks` emits the discovery header and itemized statuses
4. denied shell command in `pre_checks` fails during shell audit
5. denied `::shell` directive in the source page fails before provider launch
6. handled pre-check failure emits the handler-engagement banner exactly once
7. redirect reruns source-file reporting and shell audit for the new file
8. `--silent` suppresses status output but preserves exit behavior

## Documentation Follow-Through

When this design is implemented, the same change should update:

1. `claudine/docs/topics/validations-and-handlers.md`
2. `claudine/lib/README.md`

Both documents currently describe the harness correctly at a semantic level but do not describe the new status-based reporting contract or shell-audit preflight.

## Implementation Sequence

Recommended order:

1. add `harness/report.rs`
2. refactor `validate.rs` to return structured outcomes
3. add `harness/audit.rs` and shell-audit unit tests
4. thread original file-reference text through wrapper prompt state
5. integrate source-file reporting and shell audit into `run_harness_loop(...)`
6. integrate handler-engagement and terminal failure banners
7. update docs

This order keeps the work incremental and makes it easy to verify reporting changes without mixing them immediately with handler-loop changes.

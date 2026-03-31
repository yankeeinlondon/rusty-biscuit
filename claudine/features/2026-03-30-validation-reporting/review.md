# Validation Reporting Review

## Summary

The implementation landed the basic module split (`report.rs`, `audit.rs`, structured validation outcomes, wrapper integration), but there are still a few meaningful gaps between the design and the shipped behavior:

- harness shell-audit failures do not participate in handler recovery
- redirected attempts do not carry the new source reference cleanly
- handler-engagement reporting can be emitted when no recovery will actually occur
- validation item rendering still mixes old inline status tokens with the new `Status` UI and does not fully escape user-controlled content
- the new reporting/audit surface is light on both unit and integration coverage

## Findings

### 1. Harness shell-audit failures are terminal instead of flowing through handler resolution

References:

- `claudine/cli/src/commands/wrap/mod.rs:2184-2218`
- `claudine/lib/src/harness/audit.rs:14-150`
- `claudine/features/2026-03-30-validation-reporting/tech-design.md`

The design explicitly called out that shell-audit failures for harness-declared runtime commands should still be able to flow through the existing handler model, while source-page `::shell` failures are terminal in v1. The current loop does not do that. Once `audit_report` contains any non-source failure, the wrapper emits an unhandled failure banner and returns `Err("shell audit failed")` immediately.

That means all of these designed recovery paths are currently missing for audit failures:

- a `shell_command` validation that is denied by policy cannot trigger `retry`, `redirect`, `resume`, or `deviate`
- a denied declarative `deviate` command cannot be surfaced through typed failure handling
- a denied programmatic `handle` command never gets converted into a normal failure context

Recommendation: classify non-source audit failures into `FailureContext` values and run them through `resolve_handler(...)` the same way pre-check/post-check failures already do.

### 2. Redirected attempts keep the old `original_ref`, so source-file reporting becomes inaccurate

References:

- `claudine/cli/src/commands/wrap/mod.rs:1910-1926`
- `claudine/cli/src/commands/wrap/mod.rs:2133-2139`
- `claudine/features/2026-03-30-validation-reporting/plan.md`

`HarnessPromptState` now carries `original_ref`, but `apply_next_attempt_plan(...)` only updates `source_path` on redirect. The next loop iteration therefore reports the redirected file using the original reference string from the first attempt.

In practice that produces mismatched status output after redirect, e.g. "the file reference `<old-ref>` to the `<new-file>` file on this host".

Recommendation: when a redirect is applied, update both the active source path and the active file reference used for reporting. If preserving both matters, carry `initial_ref` and `current_ref` separately.

### 3. Handler-engagement can be reported when no handler will actually run, and can repeat within one failure episode

References:

- `claudine/cli/src/commands/wrap/mod.rs:1963-1968`
- `claudine/cli/src/commands/wrap/mod.rs:1993-1998`
- `claudine/cli/src/commands/wrap/mod.rs:2251-2272`
- `claudine/cli/src/commands/wrap/mod.rs:2445-2466`
- `claudine/cli/src/commands/wrap/mod.rs:2545-2566`

The design says to emit the handler-engagement banner once per failure episode, immediately before recovery actually begins. The current code emits the banner before calling `build_next_attempt_plan(...)`.

That is observable in at least two cases:

- when retry/resume ceilings are reached, `build_next_attempt_plan(...)` returns `Ok(None)`, but the user has already been told Claudine is "engaging registered handlers"
- if multiple failure contexts resolve to actions that cannot produce a plan, the banner can be printed multiple times in one failure episode

Recommendation: move `report_handler_engagement(...)` to the point where a concrete `NextAttemptPlan` has been produced (`Some(...)`), then emit it exactly once before `apply_next_attempt_plan(...)`.

### 4. Validation item rendering still carries old inline pass/fail tokens and does not fully escape user-controlled markup

References:

- `claudine/lib/src/harness/validate.rs:733-791`
- `claudine/lib/src/harness/report.rs:23-32`
- `claudine/lib/src/harness/report.rs:92-99`

The new reporting path renders each validation through `Status::from_prose(...)`, but `build_check_markup(...)` still injects the legacy `{{status}}` token (`✓` / `⨯`) into every message template. As a result, validation lines are now likely to show both:

- the `Status` component's circular state indicator
- the old inline check/cross token embedded in the message body

That is a UI regression relative to the design, which moved status semantics into `StatusState`.

There is also still escaping work missing:

- `build_vars(...)` interpolates file paths, command text, property names, and response needles directly into Prose markup without escaping
- `prose_escape(...)` does not escape double quotes even though `report_source_file(...)` inserts the path into `href="..."`

Recommendation:

- remove the inline glyph from validation message templates for the status-rendered path
- escape all user-controlled values before interpolation into Prose markup
- extend the escape helper to cover attribute-sensitive content like `"`

## Coverage Gaps

### Reporting helpers

`claudine/lib/src/harness/report.rs:150-174` only covers `prose_escape(...)` and the zero-count early-return cases. There is no direct test coverage for:

- source-file success/failure messages
- singular/plural grammar in discovery headers
- handler-engagement banner text
- terminal failure banner rendering
- state mapping correctness for success/info/warning/failure

### Shell audit execution

`claudine/lib/src/harness/audit.rs:153-220` only tests command collection. The planned tests for `audit_shell_commands(...)` are missing, so there is no coverage for:

- approved commands
- denied commands
- blacklisted commands
- error-message rendering
- `ShellAuditReport::all_passed()` / `failures()`

### Wrapper integration

`claudine/cli/tests/wrap_commands.rs:1491-1558` still only verifies the pre-existing "pre-check blocks provider launch" behavior. I did not find integration coverage for the validation-reporting feature itself:

- source-file reporting
- shell-audit headers and itemized results
- source-page `::shell` denial before launch
- handler-engagement banner emitted exactly once
- redirect rerunning source-file reporting and shell audit for the new file
- `--silent` suppressing the new status output

## Ergonomics And Performance

### 1. Deduplicate handler-resolution/reporting flow

The pre-check, agent-failure, inline-closure, and post-check branches all repeat the same pattern:

- resolve handler
- emit handler banner
- build next plan
- apply it or fall through to terminal failure

Extracting that into one helper would reduce duplication and eliminate the current banner-ordering bug in one place instead of four.

### 2. Carry the active source reference and raw source text through the loop

The loop currently re-reads the source file for shell audit (`read_to_string`) and separately materializes/composes it for prompt generation. Threading the current source reference and raw source text through `MaterializedHarnessPrompt` would:

- fix the redirect reporting bug cleanly
- avoid duplicate disk reads/parsing on subsequent attempts
- make "report source-file status before parse/materialization" easier to implement consistently

## Verification Note

Static review was completed from the spec/design and code. I also attempted to run the package tests with `just test` in `claudine/`, but the workspace test run is currently blocked in this environment because the installed Cargo is too old for the workspace's Rust 2024-edition manifests.

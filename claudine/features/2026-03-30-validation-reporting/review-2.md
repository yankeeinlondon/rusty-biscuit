# Validation Reporting Remaining Work

This follow-up captures only the items from `review.md` that still appear to need work after checking the current implementation and test suite.

## Remaining Items

### 1. Add explicit shell-audit coverage for the non-blacklist denial path

`audit_shell_commands(...)` is now covered for:

- approved commands
- blacklisted commands
- mixed reports
- escaping
- `all_passed()` / `failures()`

What is still missing is a direct test for the plain `ShellCommandDenied` branch, where a command is denied by policy rather than rejected by the hard blacklist.

References:

- `claudine/lib/src/harness/audit.rs`
- `claudine/lib/src/harness/shell.rs`

Recommendation:

- add a unit test that supplies shell policy inputs causing `validate_and_approve_command_parts(...)` to return `ShellCommandDenied`
- assert that the rendered audit outcome contains `denied by policy`

### 2. Add wrapper integration coverage for handler-engagement emission semantics

The code now emits the handler-engagement banner only after a concrete `NextAttemptPlan` exists, which fixes the original behavior bug.

What is still missing is integration coverage proving:

- the banner is not emitted when retry/resume ceilings are hit and no plan is produced
- the banner is emitted exactly once within a single failure episode

References:

- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/tests/wrap_commands.rs`

Recommendation:

- add a wrapper integration test where a handler matches but cannot produce a plan because the retry ceiling is reached
- assert that stderr does not claim handlers are being engaged
- add a second integration test that exercises a successful recovery path and asserts the banner appears exactly once

### 3. Add wrapper integration coverage for redirect status reporting

Redirect now updates both the active source path and the reporting reference, so the original bug appears fixed in code.

What is still missing is an end-to-end test proving that after redirect:

- source-file reporting uses the redirected file reference
- shell audit reruns against the redirected source
- output does not mix the old reference with the new path

References:

- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/tests/wrap_commands.rs`

Recommendation:

- add a harness integration test with an initial source that redirects to a second file
- assert the second attempt reports only the redirected file reference/path
- assert the redirected file's validations and shell audit are the ones that run

### 4. Add validation-reporting-specific `--silent` coverage

General wrapper silent-mode coverage exists, but I did not find an integration test that specifically proves the new validation-reporting output is suppressed under `--silent`.

What should be covered explicitly:

- source-file status output
- shell-audit headers and itemized results
- handler-engagement banners
- terminal validation failure banners

References:

- `claudine/cli/tests/wrap_commands.rs`

Recommendation:

- add one or more integration tests that trigger validation-reporting output under normal verbosity
- rerun the same scenarios with `--silent`
- assert those reporting lines are absent

### 5. Decide whether `{{status}}` remains supported in custom validation messages

The default rendering path no longer injects inline pass/fail glyphs, which matches the design. Escaping work is also in place.

However, custom message templates can still contain `{{status}}`, and the generic template renderer still substitutes it if callers provide that variable. Current harness validation rendering no longer populates `status` in the main path, so this token now appears to be legacy compatibility surface rather than an intentional feature.

References:

- `claudine/lib/src/harness/validate.rs`
- `claudine/lib/src/harness/parse.rs`

Recommendation:

- decide whether `{{status}}` should be:
  - explicitly unsupported and removed from examples/tests, or
  - intentionally preserved with documented semantics
- if unsupported, remove or update parsing/tests that still use it as a normal message-template example
- if preserved, add a targeted test and document exactly what value it expands to in status-rendered output

## Verification Note

This follow-up is based on the current repository state plus a successful `just test` run in `claudine/`.

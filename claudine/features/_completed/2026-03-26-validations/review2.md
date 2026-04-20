# Validations, Timeouts, and Handlers: Second Review

## Scope Reviewed

This review compares the current implementation against:

- `claudine/features/2026-03-26-validations/spec.md`
- `claudine/features/2026-03-26-validations/tech-design.md`
- `claudine/features/2026-03-26-validations/review.md`

Primary implementation areas reviewed:

- `claudine/lib/src/harness/`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/profile.rs`
- `claudine/cli/tests/`

## Overall Assessment

Several important issues from the first review were fixed:

- subject-specific handler keys are now canonicalized
- frontmatter post-validations now read post-run disk state
- `--frontmatter-prompt` post-checks now run after file reconciliation
- `--prompt-file` harness activation now uses composed frontmatter
- response length checks now count characters instead of bytes
- timeout classification now flows into harness outcomes

However, the feature is still incomplete relative to the spec and tech design.

The biggest remaining gap is recovery behavior. The wrapper now has a harness loop for non-inline execution, but the handler actions are still only partially applied. In practice:

- pre-check failures are still terminal
- inline `--frontmatter-prompt` still does not support recovery
- `retry` does not apply `prompt` or `set`
- `resume` validates support but does not actually resume
- `redirect` is still unimplemented
- response-based validations are still broken on legacy non-structured runs
- shell approval is still not enforced end-to-end

## Findings

### 1. Pre-check failures still bypass handler resolution

Severity: High

`pre_checks` are still treated as hard failures instead of harness failures that can be handled by `handle_{event}` or programmatic `handle`.

Observed behavior:

- initial pre-check failure exits immediately before any handler resolution
- retry attempts repeat the same direct-fail behavior

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:838`
- `claudine/cli/src/commands/wrap/mod.rs:1547`

Impact:

- pre-run validation failures cannot trigger `retry`
- pre-run validation failures cannot trigger `deviate`
- pre-run validation failures cannot trigger subject-specific handlers
- the harness still does not fully implement recovery for all failure phases described in the design

Recommendation:

- convert pre-check failures into `FailureContext` values
- run the same handler resolution path used for agent failures and post-check failures
- allow handler-driven retry / deviate / redirect from pre-check failures

### 2. `--frontmatter-prompt` still does not support recovery handlers

Severity: High

Inline execution now resolves handlers, but it still logs a warning and exits instead of applying the handler action.

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:1478`
- `claudine/cli/src/commands/wrap/mod.rs:1500`

Impact:

- `retry` is still non-functional for `--frontmatter-prompt`
- `resume` is still non-functional for `--frontmatter-prompt`
- `deviate` is still non-functional for `--frontmatter-prompt`
- `redirect` is still non-functional for `--frontmatter-prompt`

Recommendation:

- either implement the same harness attempt loop for inline mode
- or explicitly narrow the supported scope in the spec and CLI behavior

The current state claims support implicitly but does not deliver it.

### 3. `retry`, `resume`, and `set` overlays are still only partially implemented

Severity: High

The non-inline harness loop exists, but the handler payload is still discarded when the action is applied.

Observed behavior:

- `Retry.prompt_suffix` is ignored
- `Retry.set` is ignored
- `Resume.prompt` is ignored
- `Resume.set` is ignored
- `Resume` only validates support/session presence and then increments the attempt counter
- no recomposition occurs with modified frontmatter
- no provider-specific resume argv is ever built

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:1782`
- `claudine/cli/src/commands/wrap/mod.rs:1807`
- `claudine/cli/src/commands/wrap/profile.rs:258`

Impact:

- retries do not actually change the prompt as designed
- frontmatter overlay semantics from the spec are not implemented
- resume behavior is functionally equivalent to a plain retry
- provider-specific resume support exists on profiles but is unused

Recommendation:

- replace the current `Option<u32>` handler result with a typed “next attempt plan”
- include:
  - next argv
  - next prompt body or prompt suffix
  - frontmatter `set` overlay
  - redirect source path
  - resume vs fresh launch mode
  - effective timeout
- call `profile.build_resume_args(session_id)` for real resume execution

### 4. `redirect` is still not implemented

Severity: High

`redirect` is parsed and resolved, but the runtime path still errors immediately when the action is selected.

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:1874`

Impact:

- one of the core handler types from the spec remains unavailable
- redirect-based repair flows cannot work

Recommendation:

- resolve the redirect target with the same harness resolver
- re-compose the redirected markdown source
- rebuild the harness plan from the redirected document
- support both fresh-start and resume-then-redirect semantics as designed

### 5. Response-based validations are still broken on legacy non-structured execution

Severity: High

When structured streaming is unavailable, the harness still fabricates an `AttemptOutcome` with an empty `final_response`.

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:1453`
- `claudine/cli/src/commands/wrap/mod.rs:1619`

Impact:

- `response_includes` is unreliable
- `response_missing` is unreliable
- `response_length_at_least` is unreliable
- `response_length_at_most` is unreliable

This still violates the spec requirement that response checks inspect the agent’s final response text.

Recommendation:

- ensure every execution mode produces a real final response string
- use captured-output mode for non-structured providers
- parse provider-specific captured output through existing profile hooks where needed

### 6. Shell approval is still not fully enforced

Severity: High

The approval adapter exists, but runtime command parsing still bypasses it.

Observed behavior:

- `shell_command` still uses tokenizer-only parsing
- programmatic `handle` still accepts array/object commands without approval
- `deviate` still tokenizes directly
- expanded object form for `shell_command` is still unsupported

Relevant code:

- `claudine/lib/src/harness/parse.rs:299`
- `claudine/lib/src/harness/parse.rs:442`
- `claudine/lib/src/harness/parse.rs:694`
- `claudine/lib/src/harness/shell.rs:42`

Impact:

- approval guarantees from the spec are still not met
- shell commands can still bypass the intended whitelist/blacklist/approval path
- the ergonomic expanded form described in the design still does not parse

Recommendation:

- route `shell_command`, `handle`, and `deviate` through `validate_and_approve_command`
- support expanded object form for `shell_command`:

```yaml
shell_command:
  cmd: "cargo test -p claudine"
  show_stdout: true
  show_stderr: false
```

### 7. Harness-aware wrapper integration tests are still largely missing

Severity: Medium

Library-level unit coverage is decent, but there is still almost no meaningful wrapper integration coverage for the harness behavior itself.

Observed state:

- harness unit tests exist and pass
- wrapper integration tests do not cover harness flows
- `wrap_commands.rs` contains no meaningful `pre_checks` / `post_checks` / handler loop coverage

Missing or insufficient integration coverage:

1. pre-check failure with handler recovery
2. post-check failure with retry loop
3. timeout to `handle_timeout`
4. subject-specific handler matching through wrapper execution
5. real resume path on a supported provider
6. redirect flow
7. deviate flow
8. programmatic `handle` flow
9. response validations on non-structured providers
10. compose / prompt-file harness activation with composed frontmatter
11. inline `--frontmatter-prompt` harness behavior

Recommendation:

- add wrapper integration tests that exercise full harness flows, not just harness units
- test all three intended markdown-backed entry points:
  - `--prompt-file`
  - `--frontmatter-prompt`
  - `--compose`

### 8. The package test suite is still not fully green

Severity: Medium

Current test status:

- harness library tests pass
- `cargo test -p claudine --lib harness -- --nocapture` passes
- `just test` in `claudine/` still fails due to unrelated PTY test timeouts

Failing tests:

- `claudine/cli/tests/pty_tests.rs`

Impact:

- the package is not currently fully green
- this weakens release confidence for a feature that still lacks wrapper integration tests

Recommendation:

- fix or quarantine the PTY test timeouts
- make the full package suite green before treating this feature as production-ready

## Features Designed But Still Not Fully Implemented

1. Handler-driven recovery from pre-check failures
2. Full recovery support for `--frontmatter-prompt`
3. Retry prompt augmentation via `prompt`
4. Frontmatter override semantics via `set`
5. Actual provider resume execution
6. Redirect runtime behavior
7. End-to-end shell approval enforcement
8. Correct response validations on legacy execution paths
9. Strong wrapper integration coverage for the harness

## Ergonomics and Performance Recommendations

### 1. Replace “next attempt number” with a typed next-attempt plan

Current handler application only returns `Option<u32>`, which loses nearly all of the useful handler data.

Recommended shape:

- `NextAttemptPlan`
  - next attempt number
  - source path
  - recomposed prompt body
  - argv
  - stdin seed
  - frontmatter overlay
  - resume mode
  - timeout

Benefits:

- fixes correctness
- makes handler application explicit
- reduces hidden coupling
- improves testability

### 2. Unify structured and legacy execution into one outcome path

Every path should produce a real `AttemptOutcome` with:

- final response text
- session ID when available
- stderr text
- termination classification

Benefits:

- makes response validations correct everywhere
- removes one of the remaining structured vs legacy behavior gaps
- simplifies wrapper logic

### 3. Recompose on every handler-driven retry

If a handler supplies:

- `prompt`
- `set`
- `redirect`

then the wrapper should rebuild the effective document state before the next attempt.

Benefits:

- makes retry semantics correct
- preserves consistency with Darkmatter composition behavior
- ensures post-checks compare the right source state

### 4. Keep shell approval centralized

Avoid having tokenizer-only parsing paths in `parse.rs` for runtime commands.

Benefits:

- fewer policy gaps
- fewer duplicated parsing rules
- easier auditing of shell safety behavior

## Prioritized Recommendations

### Immediate

1. Handle `pre_checks` failures through the normal handler resolution path.
2. Implement actual retry/resume state mutation instead of only incrementing the attempt counter.
3. Implement `redirect`.
4. Fix response validations on non-structured execution paths by capturing real output.
5. Route all runtime commands through the shell approval adapter.

### Near-term

1. Add harness integration tests in `claudine/cli/tests/`.
2. Implement recovery support for `--frontmatter-prompt` or explicitly narrow its support.
3. Make provider resume use `build_resume_args`.
4. Make full `claudine/` package tests green again.

## Bottom Line

The second pass materially improved the implementation, and several major correctness issues from the first review were addressed.

However, the feature is still not complete enough to claim full spec/design compliance.

Most importantly:

- pre-check failures still cannot be handled
- inline mode still does not support recovery
- retry/resume still do not actually mutate execution state
- redirect is still unimplemented
- response checks still fail on non-structured runs
- shell approval is still bypassable
- wrapper integration coverage is still too light

The next step should be to finish the real recovery model end-to-end, then add wrapper integration coverage for all three markdown-backed workflows.

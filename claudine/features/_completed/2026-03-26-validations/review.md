# Validations, Timeouts, and Handlers: Implementation Review

## Scope Reviewed

This review compares the implemented code against:

- `claudine/features/2026-03-26-validations/spec.md`
- `claudine/features/2026-03-26-validations/tech-design.md`
- `claudine/features/2026-03-26-validations/plan.md`

Primary implementation areas reviewed:

- `claudine/lib/src/harness/`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/exec.rs`
- `claudine/cli/src/commands/wrap/profile.rs`
- `claudine/cli/tests/`

## Overall Assessment

The harness foundation is present:

- typed harness model exists
- parsing for validations, handlers, and timeout exists
- pre-check and post-check validation engine exists
- wrapper integration detects harness frontmatter and runs checks
- timeout-aware process termination is implemented in `exec.rs`

However, the implementation is still incomplete relative to the spec and tech design. The most important missing piece is the actual harness execution loop. Handler parsing and handler-resolution utilities exist, but they are not connected to the wrapper execution path, so the system currently behaves as a one-shot validator rather than the designed recovery harness.

There are also several correctness gaps in post-validation behavior, particularly around frontmatter comparisons, subject-specific handler matching, and shell command approval.

## Findings

### 1. Handler execution loop is not implemented

Severity: High

The wrapper never invokes:

- `resolve_handler`
- `classify_failure`
- `build_validation_failure_context`
- `build_agent_failure_context`
- `execute_deviate_command`
- `validate_resume`

Observed behavior:

- harness frontmatter is parsed once
- pre-checks run once
- snapshot is captured once
- provider runs once
- post-checks run once
- any failure exits immediately

Impact:

- `retry` is dead code
- `resume` is dead code
- `redirect` is dead code
- `deviate` is dead code
- `handle` programmatic handler is dead code
- `handle_timeout` and `handle_agent_failure` never affect behavior
- retry ceilings and recovery semantics from the spec are not implemented

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:765`
- `claudine/cli/src/commands/wrap/mod.rs:1259`
- `claudine/cli/src/commands/wrap/mod.rs:1546`
- `claudine/lib/src/harness/handlers.rs:37`
- `claudine/cli/src/commands/wrap/exec.rs:21`
- `claudine/cli/src/commands/wrap/profile.rs:251`

Recommendation:

- implement a single wrapper-side attempt loop
- carry `AttemptOutcome` and `ProcessTermination` through every path
- classify failures into timeout / agent failure / validation failure
- resolve handlers in the precedence specified by the tech design
- re-compose / re-run on retry and redirect
- validate provider resume support before attempting resume

### 2. Subject-specific handler matching is broken for file-based validations

Severity: High

Validation subject keys are normalized to resolved absolute paths:

- `file_exists`
- `dir_exists`
- `json_file_exists`
- `yaml_file_exists`
- `toml_file_exists`
- `has_write_permission`
- `file_changed`
- `file_unchanged`
- dirty-source-code roots

But subject-specific handler keys from frontmatter are stored verbatim.

Example:

```yaml
post_checks:
  file_exists: "@docs/output.md"

handle_file_exists:
  "@docs/output.md":
    retry:
      prompt: "Create the file."
```

The validation failure subject becomes something like `/repo/docs/output.md`, while the handler subject remains `"@docs/output.md"`. These never match.

Relevant code:

- `claudine/lib/src/harness/parse.rs:243`
- `claudine/lib/src/harness/parse.rs:575`
- `claudine/lib/src/harness/handlers.rs:42`

Impact:

- subject-specific YAML handlers are unreliable for the main path-oriented validations
- the precedence rules from the tech design effectively collapse to generic handlers only

Recommendation:

- normalize handler subject keys using the same resolver as validation subjects
- store canonical subject keys in `HandlerRule`
- add tests covering `@repo-relative`, relative, and absolute subject forms

### 3. Frontmatter post-validations are functionally incorrect

Severity: High

`frontmatter_prop_changed` and `frontmatter_prop_unchanged` do not inspect the post-run file state. They compare:

- pre-run captured property value
- against the same pre-run markdown object stored in the snapshot

The code comments acknowledge this is incomplete.

Relevant code:

- `claudine/lib/src/harness/validate.rs:491`

`frontmatter_prop_equals` is also inconsistent:

- snapshot capture explicitly says no pre-snapshot is needed
- evaluation still requires `snapshot.source_markdown`
- if no other frontmatter comparison rule populated it, this can become an internal error or only observe pre-run state

Relevant code:

- `claudine/lib/src/harness/validate.rs:81`
- `claudine/lib/src/harness/validate.rs:531`

Impact:

- frontmatter comparison checks do not reflect the actual on-disk post-run state
- the most important content-mutation validations are unreliable

Recommendation:

- add the source path to post-validation context or pass a post-run parsed markdown document
- parse the current on-disk file once after execution
- compare pre-value vs post-value from disk for changed/unchanged rules
- use post-run disk state for `frontmatter_prop_equals`

### 4. `--frontmatter-prompt` runs harness post-checks too early

Severity: High

For `--frontmatter-prompt`, harness post-checks run before Claudine performs its existing:

- body update validation
- frontmatter tamper detection
- frontmatter restoration
- `last_updated` rewrite

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:1259`
- `claudine/cli/src/commands/wrap/mod.rs:1394`

Impact:

- `file_changed` / `file_unchanged` can observe pre-rewrite state
- `frontmatter_prop_*` can observe stale or transient state
- post-check results for the inline markdown workflow can disagree with the actual final file written by Claudine

Recommendation:

- move harness post-check execution to the end of the inline file-reconciliation path
- run validations against the final persisted state, not the temporary agent-written state

### 5. Post-check behavior is inconsistent across execution modes

Severity: High

For non-structured legacy execution:

- prompt-file/compose path skips harness post-checks entirely
- frontmatter-prompt fabricates an `AttemptOutcome` with an empty response

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:1271`
- `claudine/cli/src/commands/wrap/mod.rs:1558`
- `claudine/cli/src/commands/wrap/mod.rs:1573`

Impact:

- `response_includes`, `response_missing`, `response_length_*` are meaningless on the inline legacy path
- post-checks do not consistently run for all supported workflows
- behavior depends on provider structured-stream capability rather than harness semantics

Recommendation:

- ensure every execution path produces a real `AttemptOutcome`
- use captured-output mode when structured stream support is unavailable
- never skip post-checks just because the provider is on the legacy path

### 6. Shell approval model is not fully enforced

Severity: High

The spec and design require runtime-detectable shell commands to go through approval/whitelist logic. That is not happening end-to-end.

Current behavior:

- `shell_command` tokenizes but does not call `validate_and_approve_command`
- `handle` programmatic commands are parsed without approval
- `deviate` commands are parsed without approval
- `execute_approved_command` ignores the timeout argument

Relevant code:

- `claudine/lib/src/harness/shell.rs:42`
- `claudine/lib/src/harness/shell.rs:181`
- `claudine/lib/src/harness/parse.rs:299`
- `claudine/lib/src/harness/parse.rs:458`
- `claudine/lib/src/harness/parse.rs:656`

There is also a parser gap:

- expanded object form for `shell_command` is not actually supported
- parser only accepts a scalar command string

Relevant code:

- `claudine/lib/src/harness/parse.rs:299`

Impact:

- command approval guarantees from the spec are not satisfied
- shell validations and recovery commands can bypass the intended policy system
- shell commands can hang indefinitely because execution timeout is ignored

Recommendation:

- route `shell_command`, `deviate`, and programmatic `handle` parsing through `validate_and_approve_command`
- implement actual timeout enforcement in `execute_approved_command`
- accept expanded `shell_command` object form with `cmd`, `show_stdout`, and `show_stderr`

### 7. Response length checks count bytes, not characters

Severity: Medium

The spec says response length checks are character-based. Current implementation uses `String::len()`, which is byte count.

Relevant code:

- `claudine/lib/src/harness/validate.rs:206`
- `claudine/lib/src/harness/validate.rs:216`

Impact:

- Unicode responses can incorrectly fail or pass

Recommendation:

- use `.chars().count()` for response length validations
- add tests with multibyte Unicode content

### 8. `--prompt-file` harness activation can miss composed frontmatter

Severity: Medium

`prompt_file.rs` already computes composed frontmatter and stores it in `ComposedPrompt.frontmatter`, but the wrapper ignores that and re-reads frontmatter from disk for harness detection.

Relevant code:

- `claudine/cli/src/commands/wrap/prompt_file.rs:65`
- `claudine/cli/src/commands/wrap/prompt_file.rs:301`
- `claudine/cli/src/commands/wrap/mod.rs:780`

Impact:

- harness keys introduced by composition can be lost
- wrapper behavior depends on source file disk frontmatter, not the actual composed prompt state

Recommendation:

- use `ComposedPrompt.frontmatter` directly when present
- avoid re-reading from disk after composition

## Features Designed But Not Fully Implemented

These feature areas exist in design/spec but are incomplete or non-functional:

1. Recovery handlers as runtime behavior
2. Provider resume integration in wrapper execution
3. Programmatic `handle` protocol in the live wrapper path
4. `redirect` flow to a different markdown document
5. `deviate` recovery execution in the live wrapper path
6. consistent timeout classification flowing into handler resolution
7. shell approval enforcement for all runtime commands
8. correct frontmatter post-state validation
9. complete post-check coverage across all supported wrapper modes

## Test Coverage Review

### What is covered reasonably well

Library unit tests exist for:

- timeout parsing
- path resolution
- harness frontmatter parsing
- individual validation functions
- handler precedence helpers
- shell helper unit tests

This gives decent local confidence in isolated pieces.

### What is missing

There is very little meaningful end-to-end test coverage of the actual harness behavior through the wrapper.

Missing or insufficient integration coverage:

1. `pre_checks` causing wrapper failure before launch
2. `post_checks` across all three supported markdown-backed workflows:
   - `--prompt-file`
   - `--frontmatter-prompt`
   - `--compose`
3. timeout classification feeding handler resolution
4. `retry` loop behavior including retry ceilings
5. `resume` behavior on supported and unsupported providers
6. `redirect` behavior to another markdown document
7. `deviate` command execution
8. programmatic `handle` command behavior
9. subject-specific handler matching with normalized file references
10. shell approval enforcement and shell-command timeout behavior
11. response validations in legacy non-structured provider paths
12. Unicode response length behavior
13. frontmatter property comparison against actual post-run disk state
14. harness behavior when composition injects frontmatter keys

### Existing wrapper tests are focused elsewhere

Current wrapper integration tests mostly cover the earlier frontmatter-prompt behavior:

- file body updates
- frontmatter restoration
- avoiding overwrite on failure

Relevant existing tests:

- `claudine/cli/tests/wrap_commands.rs:1281`

These are useful, but they do not validate the new harness feature set described in the spec.

### Package test status

I ran `just test` in `claudine/`.

Observed result:

- library tests passed
- CLI/package test run failed due to unrelated PTY tests

Failing tests:

- `claudine/cli/tests/pty_tests.rs`

This does not invalidate the harness review, but it does mean the package test suite is not currently fully green and therefore is not a strong release gate for this feature.

## Ergonomics and Performance Recommendations

### 1. Unify execution into one attempt loop

This is the biggest improvement for both correctness and maintainability.

Recommended shape:

- one orchestration loop for all markdown-backed workflows
- one attempt counter
- one `AttemptOutcome`
- one termination classification path
- one handler-resolution path
- one final post-check path against final persisted state

Benefits:

- removes duplicated logic between structured and legacy flows
- makes handler support practical
- ensures timeout and agent-failure semantics are consistent everywhere

### 2. Canonicalize subject keys once

Resolve and store canonical subject keys at parse time for both:

- validation rules
- subject-specific handler rules

Benefits:

- fixes correctness
- simplifies handler resolution
- avoids repeated path-shape branching

### 3. Parse post-run markdown once per attempt

For workflows touching markdown files:

- read final on-disk file once
- build one parsed markdown representation
- feed that to all frontmatter-related post-checks

Benefits:

- correct behavior
- less repeated I/O and reparsing
- cleaner validator API

### 4. Make shell-command execution truly bounded

`execute_approved_command` should enforce timeout rather than ignore it.

Benefits:

- avoids hanging harness jobs
- aligns with the spec
- makes shell-based validation safer

### 5. Complete `has_write_permission`

The current implementation only tests OS-level open-for-write behavior. The design expects provider/sandbox-awareness too.

Recommended improvements:

- handle creation of nonexistent files in writable directories
- combine OS writability with wrapper/provider policy checks where possible
- give clearer error messages that distinguish filesystem permission failure from sandbox policy denial

## Prioritized Recommendations

### Immediate

1. Implement the actual handler/retry/resume/redirect/deviate loop in the wrapper.
2. Fix frontmatter post-validation to read actual post-run disk state.
3. Move inline `--frontmatter-prompt` harness post-checks to after file reconciliation and `last_updated` write.
4. Wire process termination info into `AttemptOutcome` instead of hard-coding `Completed`.
5. Canonicalize subject-specific handler keys.

### Near-term

1. Enforce shell approval and timeout behavior for all runtime commands.
2. Ensure post-checks run consistently in legacy non-structured execution paths.
3. Use composed frontmatter directly for `--prompt-file` harness activation.
4. Fix response length checks to count characters, not bytes.

### Testing

1. Add wrapper integration tests for all handler types.
2. Add timeout-to-handler tests.
3. Add subject-specific handler matching tests.
4. Add frontmatter property post-state tests.
5. Add shell approval / timeout tests.
6. Add compose + prompt-file harness activation tests using composed frontmatter.
7. Add Unicode response validation tests.

## Bottom Line

The current implementation delivers the harness parser and a basic validation engine, but not the full harness behavior described in the spec and design. The feature is not yet complete enough to be considered a full implementation of “validations, timeouts, and handlers.”

Most importantly:

- handlers are parsed but not executed
- recovery flow is not wired
- frontmatter post-validations are incorrect
- shell approval guarantees are incomplete
- end-to-end tests for the harness behavior are still missing

The next step should be to finish the execution loop first, then fix post-state validation correctness, then add the missing integration coverage.

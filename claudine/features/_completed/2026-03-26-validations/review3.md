# Validations, Timeouts, and Handlers: Third Review

## Scope Reviewed

This review compares the current implementation against:

- `claudine/features/2026-03-26-validations/spec.md`
- `claudine/features/2026-03-26-validations/tech-design.md`
- `claudine/features/2026-03-26-validations/review.md`
- `claudine/features/2026-03-26-validations/review2.md`

Primary implementation areas reviewed:

- `claudine/lib/src/harness/`
- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/profile.rs`
- `claudine/cli/tests/wrap_commands.rs`

## Overall Assessment

This implementation pass closed most of the major issues from the first two reviews.

Notable improvements:

- the harness now has a real execution loop across markdown-backed flows
- pre-check failures can now enter handler resolution
- `retry` now applies prompt augmentation and `set` overlays
- `resume` now uses provider-specific resume argv
- `redirect` now works in the execution loop
- legacy non-structured runs now use captured output for response validations
- inline `--frontmatter-prompt` recovery is now exercised by integration tests
- wrapper integration coverage is materially stronger
- the package test suite is now green, with PTY smoke tests explicitly ignored rather than failing

At this point the remaining issues are narrower. I did not find a new blocker on the same level as the prior reviews, but there are still a few spec/design mismatches and some remaining coverage gaps.

## Findings

### 1. Shell approval is wired, but not with source-aware policy resolution

Severity: Medium

Runtime shell validation is now routed through the approval adapter, but the wrapper always constructs `ShellApprovalOptions::default()`, which means:

- `policy_root` is `None`
- no approval handler is attached
- policy resolution falls back to `ComposeSource::Unknown`

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:870`
- `claudine/cli/src/commands/wrap/mod.rs:2643`
- `claudine/lib/src/harness/shell.rs:87`

Impact:

- harness runtime commands do not reliably consult repo-local approval files
- the implementation still does not satisfy the design goal of reusing Darkmatter’s approval model in a source-aware way
- behavior may differ depending on ambient/global policy files rather than the prompt’s repo context

Recommendation:

- build `ShellApprovalOptions` with a meaningful `policy_root`
  - repo root if available
  - otherwise the source document directory
- if interactive approval is supported in this path, pass the approval handler through as well

### 2. `redirect { resume: true }` silently downgrades to a fresh redirect when resume is impossible

Severity: Medium

When `redirect.resume` is set, the code only carries the prior session ID forward if `profile.supports_resume()` is true. Otherwise it silently falls back to a fresh redirect.

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:2556`
- `claudine/cli/src/commands/wrap/mod.rs:2569`

Impact:

- an explicit request for resume semantics can quietly degrade into a fresh-context retry
- users get different behavior than they asked for without an explicit warning or error

Recommendation:

- if `resume: true` is requested, validate it the same way `Resume` does
- fail clearly when the provider cannot resume or when no session ID is available

### 3. `has_write_permission` is still only an OS-level open-for-write check

Severity: Medium

The implementation still only attempts `OpenOptions::new().write(true).open(file)`.

Relevant code:

- `claudine/lib/src/harness/validate.rs:309`

Impact:

- it fails for non-existent files that could validly be created in a writable parent directory
- it does not reflect the spec text about checking the agent config / write access model
- it does not distinguish filesystem permission failure from sandbox/policy denial

Recommendation:

- handle the non-existent-file case by checking parent directory writability
- distinguish “cannot create here” from “cannot write existing file”
- if provider/sandbox policy can be queried, incorporate that into the validation result or at least into the error message

### 4. `say` remains parsed but completely unused

Severity: Low

The handler model still parses `say`, but the execution path ignores it everywhere.

Relevant code:

- `claudine/lib/src/harness/model.rs`
- `claudine/lib/src/harness/parse.rs`
- `claudine/cli/src/commands/wrap/mod.rs:2500`
- `claudine/cli/src/commands/wrap/mod.rs:2530`
- `claudine/cli/src/commands/wrap/mod.rs:2560`
- `claudine/cli/src/commands/wrap/mod.rs:2588`

Impact:

- one of the design-defined handler affordances is still not implemented
- the behavior does not match the design note that `say` should route through best-effort speech/TTS

Recommendation:

- either implement best-effort speech dispatch
- or explicitly document `say` as deferred / unsupported in this release

## Test Coverage Review

Coverage is much better than the previous review. The most important wrapper flows are now represented in `claudine/cli/tests/wrap_commands.rs`, including:

- redirect after pre-check failure
- retry with prompt suffix and `set`
- provider resume args
- legacy captured-output response validation
- inline retry recovery

The remaining notable gaps are:

1. No end-to-end wrapper test for programmatic `handle`
2. No end-to-end test for shell approval / whitelist behavior with harness commands
3. No test coverage for `redirect.resume: true` failure behavior on unsupported providers or missing session IDs
4. No test coverage for `has_write_permission` semantics around non-existent but creatable files
5. No test coverage for `say` behavior, which is unsurprising because it is still unimplemented

## Ergonomics and Maintainability Recommendations

### 1. Remove the old dead handler helper path

There is still an older `apply_handler_action` helper in `wrap/mod.rs` after the new `run_harness_loop` / `build_next_attempt_plan` flow. It appears to be superseded and now only adds maintenance noise.

Recommendation:

- delete the obsolete helper path once you are confident nothing references it

### 2. Centralize shell-option construction

`ShellApprovalOptions::default()` is currently created in multiple places without repo/source context.

Recommendation:

- add a single helper that derives shell approval options from:
  - repo root
  - source path
  - interactive approval capabilities

This will make the approval behavior easier to reason about and harder to regress.

## Prioritized Recommendations

### Immediate

1. Make shell approval source-aware by passing a real `policy_root`.
2. Validate `redirect.resume: true` instead of silently falling back.
3. Improve `has_write_permission` so it handles creatable paths and produces more precise errors.

### Near-term

1. Decide whether `say` is in scope for this feature and either implement it or document it as deferred.
2. Add wrapper integration coverage for programmatic `handle`.
3. Add harness shell-policy tests that verify repo-local whitelist/blacklist behavior.

## Bottom Line

This pass substantially improved the feature. The big architectural gaps from the earlier reviews are largely closed, the wrapper tests now exercise the main recovery flows, and the package test suite is green.

The remaining issues are narrower and mostly about polish, correctness at the edges, and finishing a few spec-defined details:

- shell approval is not yet source-aware
- redirect+resume semantics are too forgiving
- `has_write_permission` is still underpowered
- `say` is still unimplemented
- programmatic handler and shell-policy coverage are still light

The feature now looks close to complete, but it still needs one more tightening pass if the goal is full spec/design fidelity rather than “mostly implemented.”

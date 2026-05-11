# Validations, Timeouts, and Handlers: Fourth Review

## Scope Reviewed

This follow-up review checks the current implementation against the remaining
suggestions from:

- `claudine/features/2026-03-26-validations/review3.md`

Primary areas re-checked:

- `claudine/cli/src/commands/wrap/mod.rs`
- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/lib/src/harness/parse.rs`
- `claudine/lib/src/harness/validate.rs`
- `claudine/lib/src/harness/handlers.rs`
- `claudine/lib/src/composition/preflight.rs`
- `claudine/cli/tests/wrap_commands.rs`

Validation performed:

- traced the previously flagged code paths
- ran `just test` in `claudine/` and confirmed the package suite is green

## Overall Assessment

Most of the concrete issues from `review3.md` are now fixed:

- shell approval now resolves a source-aware `policy_root`
- `redirect { resume: true }` now validates resume capability instead of silently downgrading
- `has_write_permission` now handles creatable missing files and provider policy probes
- `say` is now wired through best-effort speech dispatch
- the old dead `apply_handler_action` path is gone

What remains is narrower, but still important: the shell-approval/preflight path
is not yet coherent end-to-end. The main correctness risk is no longer the
handler actions themselves; it is that approval discovery, approval prompting,
and runtime reuse are still split across multiple stages in a way that will be
fragile once interactive approval is fully enabled.

## Remaining Findings

### 1. High: interactive shell approval is still not wired in the Claudine wrapper paths

The source-aware `policy_root` part of the `review3.md` shell-approval
recommendation landed, but the second half did not: Claudine still always builds
`ShellApprovalOptions` with `approval_handler: None`.

Evidence:

- `claudine/cli/src/commands/wrap/mod.rs:170-178`
- `claudine/lib/src/composition/preflight.rs:106-115`

Impact:

- unwhitelisted commands still hard-fail instead of prompting
- the designed approval flow still does not exist in wrapper/compose execution
- current behavior only works for already-whitelisted commands

Suggestion:

- wire a real interactive approval handler into the wrapper/compose entrypoints
- keep `policy_root` derivation as-is, but stop treating approval as
  whitelist-only

### 2. High: harness parsing still performs command approval before preflight runs

The current implementation still approves harness runtime commands while parsing
frontmatter. `parse_runtime_command()` and `parse_runtime_command_parts()` call
`validate_and_approve_command*()` whenever `shell_options` are supplied, and the
wrapper passes `Some(&shell_options)` into `parse_harness_plan_with_shell()`
before `resolve_shell_approvals()` is called.

Evidence:

- `claudine/lib/src/harness/parse.rs:565-570`
- `claudine/lib/src/harness/parse.rs:912-929`
- `claudine/cli/src/commands/wrap/composition.rs:426-454`
- `claudine/cli/src/commands/wrap/mod.rs:1124-1141`

Impact:

- preflight is not the single approval authority
- once interactive approval is wired, this path is likely to duplicate prompts
  or prompt too early
- discovery and authorization remain coupled in the harness path

Suggestion:

- make harness parsing discovery-only
- defer approval to preflight/runtime audit, using a shared approved-command
  state rather than approving during parse

### 3. Medium: harness preflight results are still discarded instead of being carried into runtime

Claudine now performs a dedicated preflight pass for harness commands, but the
result is not preserved. The composition wrapper only logs the count, the plain
wrapper binds the result to `_harness_preflight`, and `run_harness_loop()`
later rebuilds an audit from the raw source file and checks policy again.

Evidence:

- `claudine/cli/src/commands/wrap/composition.rs:447-460`
- `claudine/cli/src/commands/wrap/mod.rs:1134-1141`
- `claudine/cli/src/commands/wrap/mod.rs:2234-2247`

Impact:

- future `AllowOnce` approvals would be lost across the runtime handoff
- runtime can diverge from preflight because the second pass re-reads raw source
  text instead of reusing the preflight result
- the implementation does extra work while still not establishing a single
  trusted approval pass

Suggestion:

- carry the preflight-approved command set into the harness loop
- make runtime audit enforce “must already be pre-approved” rather than
  re-running discovery and approval from scratch

## Coverage Gaps

- `claudine/cli/tests/wrap_commands.rs` still has no end-to-end coverage for
  interactive approval prompting, persisted approvals, or session-local
  `AllowOnce` behavior.
- There is still no integration test proving that harness preflight approvals
  are preserved across retries/redirects because that handoff does not exist yet.
- Programmatic `handle` coverage is still mostly unit-level; there is not yet a
  wrapper-level test that exercises it together with shell approval/preflight.

## Bottom Line

The `review3.md` action items around resume validation, writability checks,
`say`, and handler-loop cleanup are largely complete.

The remaining work is concentrated in shell approval:

1. add the actual interactive approval handler
2. stop approving harness commands during parse
3. preserve preflight approval state into runtime

Once those three pieces are aligned, this feature will be much closer to the
intended design instead of a mostly-correct implementation with split approval
authority.

# Preflight Review 3

Reviewed against:

- `claudine/features/2026-04-01-preflight/spec.md`
- current Claudine/Darkmatter implementation

Validation performed:

- Read the updated preflight, harness, and wrapper code paths.
- Ran targeted tests:
  - `cargo test -p claudine composition::preflight`
  - `cargo test -p darkmatter shell_expansion::discovery`
  - `cargo test -p claudine-cli --test wrap_commands -- --nocapture`
- Rechecked the two previously reported regressions against the rebuilt `target/debug/claudine` binary:
  - `compose --interactive` now shows the shell approval prompt with real file/line provenance.
  - `inline-compose` no longer re-audits a `::shell` hidden by a false `::block` in composition mode.

## Findings

No confirmed blocking regressions remain in the primary preflight path from the previous review. The main previously reported issues appear fixed:

- `compose` / `inline-compose` now thread `args.interactive` into preflight shell options.
- approval prompts now carry real provenance instead of `dummy:0`.
- composition-mode harness loops no longer raw-parse source-page `::shell` directives, avoiding the false-`::block` mismatch.

## Remaining Risk

### Medium: redirect/retry flows can still trigger fresh shell-approval prompts after the overall workflow has already started

This is an inference from the current control flow rather than a reproduced bug.

`run_harness_loop()` still re-runs shell audit on each loop iteration, and `audit_shell_commands()` still delegates to `validate_and_approve_command_parts()`, which will invoke the live `approval_handler` whenever a command is not already whitelisted or cached. That means a redirect to a different source file, or any later loop iteration that introduces a new auditable shell command, can still produce a new approval prompt mid-workflow.

Evidence:

- `claudine/cli/src/commands/wrap/mod.rs:2251-2266`
- `claudine/lib/src/harness/audit.rs:100-103`
- `claudine/lib/src/harness/shell.rs:178-180`

Why this matters:

- The spec’s intended contract is that all shell approvals are resolved before the provider workflow begins, so no further shell-related prompting is needed once execution is underway.
- The current implementation appears to satisfy that for the initial preflight, but not necessarily for later redirect/retry branches that surface new commands.

Suggested fix:

- Decide whether redirect/retry should be treated as a new preflight boundary or whether all reachable redirect targets must be scanned up front.
- If the contract should remain “no more shell prompts after start,” then later harness-loop audits should operate in a deny-only mode using an already-frozen approval set rather than the live interactive handler.

## Coverage Gaps

- There is still no CLI integration test that exercises the interactive preflight prompt path end-to-end for `compose` or `inline-compose`. The library coverage is better, but the CLI path was previously broken and remains unguarded by integration tests.
- There is still no integration test that locks in the false-`::block` composition/harness regression that was fixed in code. The manual repro now passes, but it should be converted into a test.
- There is no regression test for the redirect/retry scenario described above, so the remaining risk is currently unverified either way.

## Suggestions

- Add one `claudine-cli` integration test that launches `compose --interactive` under a PTY, answers `3`, and asserts that the prompt displays the real source file and line number.
- Add one `inline-compose` harness integration test with a `::shell` hidden by a false `::block` to prevent the raw-source audit regression from reappearing.
- Add one harness redirect test that proves either:
  - no new prompt occurs after the first preflight, or
  - redirects intentionally establish a new approval boundary and that behavior is documented explicitly.

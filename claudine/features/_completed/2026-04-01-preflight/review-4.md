# Preflight Review 4

Reviewed against:

- `claudine/features/2026-04-01-preflight/spec.md`
- current Claudine/Darkmatter implementation

Validation performed:

- Read the updated preflight, harness, and wrapper control flow.
- Ran targeted tests:
  - `cargo test -p claudine composition::preflight`
  - `cargo test -p darkmatter shell_expansion::discovery`
  - `cargo test -p claudine-cli --test wrap_commands -- --nocapture`
- Rechecked the previously flagged behaviors:
  - interactive `compose` now reaches the real approval prompt path
  - composition-mode harness loops no longer raw-audit hidden `::shell` directives
  - redirect/retry iterations now freeze shell approvals into deny-only mode

## Findings

No confirmed functional regressions remain in the preflight implementation.

The last remaining risk from `review-3.md` appears addressed:

- `CachedHarnessLoopContext::freeze_shell_approvals()` now strips the interactive approval handler after the initial composition-mode preflight, so later harness-loop iterations operate in deny-only mode.
- `run_harness_loop()` applies that freeze after the first audit pass for non-passthrough modes.
- `harness::shell` now includes a unit test proving cached commands still pass after freeze while new commands are denied without prompting.

Relevant code:

- `claudine/cli/src/commands/wrap/mod.rs:313-321`
- `claudine/cli/src/commands/wrap/mod.rs:2340-2347`
- `claudine/lib/src/harness/shell.rs:540-570`

## Remaining Recommendations

### Low: add CLI integration coverage for the final approval-boundary behaviors

The implementation now looks aligned with the spec, but the strongest remaining gap is end-to-end CLI coverage for the exact behaviors that regressed during this review cycle.

Recommended additions:

- Add a PTY-backed `claudine-cli` integration test for `compose --interactive` that answers `3` and asserts the approval prompt shows the real source file and line number.
- Add a composition+harness integration test proving a `::shell` hidden inside a false `::block` does not fail shell audit in `inline-compose`.
- Add a redirect/retry integration test proving that after the first composition-mode preflight, newly introduced shell commands are denied without a second interactive prompt.

## Conclusion

At this point I do not have further code-change recommendations beyond the remaining integration coverage. The implementation appears consistent with the current preflight design.

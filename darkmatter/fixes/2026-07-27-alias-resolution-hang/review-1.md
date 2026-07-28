---
$schema: feature-review.yaml
ready: true
agent: codex/default
created: 2026-07-28T02:05:17+00:00
spec: 2026-07-27-alias-resolution-hang/spec.md
implemented: false
description: A **fix** review of `2026-07-27-alias-resolution-hang/spec.md`
fix: 2026-07-27-alias-resolution-hang/review-1.md
---

# Review 1: Alias Resolution Hang

## Verdict

Ready for production.

The implementation removes every automatic shell-alias lookup from preflight
and execution, preserves the authored executable and arguments through approval
and execution, retains the typed `CommandNotFound` diagnostic, and removes the
public alias-only API together with its known downstream Claudine use. The
regression suite verifies the process-spawn, shell-configuration, command
identity, diagnostic, and real-PTY hang contracts at the appropriate levels.

## Findings

No findings.

## Requirement-to-verification assessment

| Requirement | Strongest verification found | Assessment |
|---|---|---|
| R1 — Alias lookup must not spawn a child process | Level 1: `compose_and_preflight_never_spawn_the_user_shell` installs a recording `$SHELL` fixture, exercises preflight and composition, asserts that it never ran, and requires the typed missing-command result. Level 2: `level2_compose_shell_directive_completes_in_background_process_group` runs `md compose` in a tmux PTY with job control and a background process group. | Appropriate. The no-spawn invariant is deterministic in process; the original terminal/process-group hang is verified through a real terminal. |
| R2 — Preflight and execution resolve commands identically | Level 1: `preflight_and_runtime_report_the_authored_executable` covers a single command and both supported multi-action chain operators, checking the authored executable and argv in preflight and the same executable in the runtime error. Source inspection confirms both paths now pass through the parsed command without alias rewriting. | Appropriate for command-resolution and approval data semantics; no terminal encoder or renderer behavior is involved. |
| R3 — Composition must not execute interactive shell configuration | Level 1: the recording shell fixtures source an rc sentinel if invoked; the no-spawn and configuration-independence tests assert that neither the fixture nor rc sentinel runs. | Appropriate. |
| R4 — Missing commands must remain observable | Level 1: the integration tests destructure `MarkdownError::ShellExpansion` to require `ShellExpansionError::CommandNotFound` naming `nonexistent_command_xyz`. Level 2: the CLI regression requires a nonzero exit and a user-visible command-not-found diagnostic naming the authored executable. | Appropriate. Level 1 pins the typed library contract; Level 2 pins the real-terminal CLI outcome. |
| R5 — Behavior must not depend on terminal or shell configuration | Level 1: `compose_result_is_independent_of_shell_configuration` compares results under two distinct `$SHELL` fixtures and asserts no shell or rc side effect. Level 2: the tmux test supplies a controlling terminal, enables job control with `set -m`, backgrounds composition into its own process group, and pins a bash-family shell. | Appropriate. The POSIX terminal/job-control portion needs Level 2 and has it; shell-state determinism remains a Level 1 semantic check. |
| Public alias-resolution API and downstream cleanup | Compile-time source removal of `alias.rs`, `ResolvedAlias`, `resolve_alias`, and `ShellApprovalRequest::alias_name`; Claudine's constructor is updated in the same commit. Repository search finds no remaining production reference to those shell-alias symbols. | Appropriate. |

No requirement is verified at an inappropriately low level. Level 3 is not
applicable because the fix specifies no physical keyboard or mouse behavior.
Browser and Real tiers are likewise not applicable.

## Implementation review

The production change follows the selected direct-executable design without
adding a compatibility shim or another process-management abstraction:

- `prepare_directive` now retains the parsed `ShellDirective` and displays its
  authored command.
- preflight collection uses the parsed executable and arguments directly for
  body directives, frontmatter shell values, and recursively collected content;
- the executor remains the single resolution authority through `which::which`
  and returns the existing typed error on failure;
- alias-only approval metadata and terminal prompt output are removed; and
- user documentation and the Darkmatter skill state the new direct-execution
  contract.

This is simpler and faster than the removed lookup and avoids platform-specific
process handling. I found no additional ergonomic or performance change that
belongs in this fix.

## Verification evidence and review limits

- Commit `68280a9cd` records the Darkmatter build, Level 1, lint, and Level 2
  gates as green; a real-terminal run is explicitly recorded.
- The same commit records the new Level 2 regression as observed failing before
  the production fix and records the existing missing-command case improving
  from 0.472 seconds to 0.060 seconds.
- Follow-up commit `b9be80976` independently documents the tmux test's
  approximately eight-second executed runtime versus a millisecond-scale silent
  skip and adds backend-required enforcement to the shared Level 2 recipe.
- Sniff identifies `darkmatter`, `darkmatter-cli`, and Claudine as the directly
  relevant implementation scope. Package-level dependency discovery lists
  additional Darkmatter consumers, but repository-wide symbol search finds no
  additional consumer of the removed alias API.
- `git diff --check` reports no whitespace errors in the implementation commit.

This review could not independently rerun the Rust gates: the prescribed `just`
binary was absent from the session's default `PATH`, and a direct nextest attempt
could not resolve the crates.io index under restricted network access. GitNexus
reported its index as stale and then failed to open the indexed repository for
`impact` and `detect-changes`; Sniff discovery and repository-wide symbol search
were used as the available scope checks. These environment limitations do not
contradict the recorded executed Level 1/Level 2 evidence or the inspected source
and test behavior.

## Production readiness

The fix is production ready.

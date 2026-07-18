---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-17T21:09:58-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-6.md
previous: 2026-07-13-proxy-with/review-5.md
---

# Review 6: Proxy With

## Verdict

The feature is **not ready for production**. Review 5's compile and surfaced-
handoff wiring failures have been addressed: the command-owned coordinator now
consumes terminal and sequence handoffs, and the production launch pipeline
reselects the target provider/profile/binary and rebuilds MCP, argv, environment,
and system-prompt state. A dedicated Level 2 CI job and a call-site structural
guard have also landed.

Four release blockers remain. Dry run deliberately executes lifecycle side
effects despite this specification explicitly preserving a no-lifecycle,
side-effect-free contract. Resume incompatibility is still a latent branch that
cannot be reached by a canonical refresh. The Level 2 equivalence matrix does
not verify all context/launch facets the specification requires, and route-
equivalent typed diagnostic rendering remains Level 1 only. The latter two are
wrong-level gaps under this review's rigor policy and therefore independently
prevent a production-ready verdict.

## Findings

### 1. High: `--dry-run` executes lifecycle side effects in direct conflict with the specification

The specification says dry run remains side-effect-free and fires no lifecycle
events (`spec.md:85-87`), repeats that changing dry run into lifecycle simulation
is a non-goal, and requires an L2 case proving no lifecycle side effect
(`spec.md:1052`). The implementation and tests deliberately establish the
opposite behavior. `wrap_compose_preflight.rs:300-310` says the no-lifecycle
half is false, while
`level2_lifecycle_dispatch.rs:416-438` runs a real-terminal dry run and asserts
that `initialize`, `blocked`, and `finalize` append markers to `events.log`.
Those file writes are observable side effects, not merely internal lifecycle
bookkeeping.

This is especially risky for a command intended for CI rehearsal: lifecycle
stacks can contain mutation and effect actions, so users cannot rely on
`--dry-run` to leave the workspace unchanged. Move the dry-run seam before
lifecycle dispatch, or explicitly ratify a different dry-run contract in the
spec before calling the feature complete. Under the current review authority,
the implementation must follow the existing specification.

**Verification level:** Level 2 exists, but it verifies the wrong behavior.
Level 3 is not applicable.

### 2. High: AC15's resume-incompatibility behavior is unreachable

Acceptance criterion 15 requires a canonical resume refresh to reject a live
session when any non-renegotiable launch facet changes, name the changed facets,
and recommend retry (`spec.md:914-917`). The current L2 test documents that no
same-document refresh can change any key facet and calls the refusal a latent
guard (`level2_lifecycle_control.rs:891-904`). The acceptance map makes the same
admission (`notes/acceptance-map.md:29-34,92`), and `session_key.rs:20-35` still
uses invocation-canonical argv while leaving provider-specific identity empty.

The compatible resume happy path is valuable but proves only that a dropped
resume-only flag does not cause a false rejection. It cannot prove the required
refusal, non-launch, facet list, or rendered retry advice. Rebuild the complete
typed launch bundle on each retry/resume fresh-read boundary, compare that bundle
with the opener's key, and add L2 refusal rows for every compatibility facet.

**Verification level:** facet projection is Level 1 and compatible resume is
Level 2; the required incompatible live path has no runnable test. This is a
wrong-level gap and an implementation gap. Level 3 is not applicable.

### 3. High: the required direct/proxy context and launch matrix remains incomplete

AC9 requires `ctx.area`, `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL`
to match direct execution across body, frontmatter, and lifecycle surfaces. The
acceptance map marks it complete while citing L2 evidence only for `ctx.area`,
`ctx.os`, and `env.MODEL` (`notes/acceptance-map.md:86`). The principal probe
stamps only `ctx.area` and `ctx.os`
(`level2_lifecycle_control.rs:2814-2837`); it does not exercise `ctx.agent`,
`ctx.model`, or `env.AGENT`, nor all three required composition surfaces.

AC10 and the spec's L2 strategy additionally require target-specific MCP tags,
argv, effective environment, interactivity, structured mode, and dispatch
configuration. The matrix has good provider, model, child-CWD, CLI system-
prompt, cross-repository, and output-routing rows, but its own comments say the
target-driven profile/binary, MCP injection, and argv cases are not represented
(`level2_lifecycle_control.rs:3985-3994`), and the acceptance map leaves AC10
partial (`notes/acceptance-map.md:87`). The production coordinator appears more
complete than those comments suggest: after target preparation it selects the
provider profile/binary and rebuilds MCP and argv in
`wrap/composition/pipeline.rs:267-269,399-483,552-560`. That makes this at least
a verification and documentation gap, not evidence that the complete behavior
works.

Extend the direct-versus-proxy L2 matrix with target-driven rows for every named
AC9/AC10 facet, including a provider-switch case that asserts the selected
binary/entrypoint and MCP injection. Do not use a pinned provider for facets
whose purpose is to prove target-owned selection.

**Verification level:** Level 2 is partial and Level 1 projection/state tests
cover some omitted facets. The specification explicitly requires Level 2 for
these user-observable provider-launch outcomes. Level 3 is not applicable.

### 4. High: route-equivalent typed diagnostics are verified only in process

AC28 requires the same target failure and actionable rendering across direct,
initialize-proxy, and terminal-recovery-proxy routes. The spec explicitly places
typed failure identity and rendered diagnostics in the L2 equivalence matrix
(`spec.md:1010`). The acceptance map marks AC28 complete using only preparation-
service Level 1 tests (`notes/acceptance-map.md:105`). Those tests can compare
Rust variants, but they cannot prove that the shipped binary preserves the same
error through each coordinator/harness boundary or renders the same terminal
diagnostic.

Add a three-route L2 matrix for at least schema failure, invalid overlay, and a
typed target-preparation failure. Assert exit status, error identity fields,
plain pane text, source attribution, and absence of duplicate rendering.

**Verification level:** Level 1 only; Level 2 is required. Level 3 is not
applicable.

### 5. Medium: sign-off artifacts contradict their own remaining gaps and the live pipeline

The acceptance map's headline says all 30 criteria map to passing tests and that
AC9/10 and AC15 are unblocked (`notes/acceptance-map.md:7-12`), yet the same file
marks AC10 and AC15 partial and describes the resume refusal as latent
(`notes/acceptance-map.md:87,92`). It also says provider-switch launch facets
remain borrowed (`notes/acceptance-map.md:24-28`), while the surfaced command
path now re-enters the production selection/MCP/argv pipeline. Similar stale
claims remain in `target_launch.rs:21-26` and the L2 test commentary.

Reconcile the acceptance map, plan, topic docs, skill copy, source comments, and
tests after findings 1-4 are resolved. The sign-off headline must count partial
criteria as partial, and fallback-path limitations must not be presented as
limitations of the surfaced command coordinator when they are not.

## Verification-level audit

| User-observable requirement | Strongest present | Assessment |
|---|---:|---|
| `proxy.with` parsing, typed recursion, precedence, null/shallow semantics | Level 1 | Appropriate for pure data semantics |
| Target initialize, reread, loop ownership, terminal/sequence handoff consumption | Level 2 | Appropriate and materially improved |
| Inline final-target rewrite and overlay survival across retry/resume/loop | Level 2 | Appropriate |
| Shell audit and approved-bytes-equal-executed-bytes | Level 2 | Appropriate |
| Handoff-refusal ordering and `err.*` visibility | Level 2 | Appropriate |
| Overlay non-disclosure in rendered status | Level 2 pane capture | Appropriate |
| Dry-run has no lifecycle side effects | Level 2 asserts lifecycle writes | **Implementation contradicts requirement** |
| Full AC9 context equivalence across body/frontmatter/lifecycle | Partial Level 2 | **Mismatch** |
| Full AC10 target launch equivalence | Partial Level 2 | **Mismatch** |
| Resume incompatible-session refusal | Level 1 projection only; live branch unreachable | **Mismatch / incomplete** |
| Same typed failure and rendered diagnostic across all proxy routes | Level 1 | **Mismatch** |
| OS keyboard/input encoder behavior | None | Not applicable; no Level 3 requirement |

## Validation performed

- `just test` — catalog types passed 21/21, the Claudine library passed
  3,527/3,527 (7 skipped), and the contract passed 47/47 (5 skipped). The command
  was stopped with exit 130 while compiling `claudine-cli` after it exceeded the
  non-interactive 60-second command guard; CLI and generator results were not
  produced by this run.
- `just test-cli run_harness_loop_call_sites` — attempted twice; both attempts
  were stopped with exit 130 at the same guard while compiling `libduckdb-sys` /
  `claudine-cli`, before tests ran.
- `just test-l2` — not run because the CLI test target did not finish compiling
  within the non-interactive command guard. The branch now contains a dedicated
  Linux/tmux Level 2 CI job, but this review does not claim a local L2 pass.
- `just lint` — not run because it would repeat the same unfinished CLI build.

The completed green L1 suites support the pure state, overlay, and library
transition contracts they exercise. They do not close the wrong-level or
unreachable user-facing requirements above.

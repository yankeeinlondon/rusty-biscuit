---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-18T02:00:59-07:00
spec: 2026-09-15-initialize-after-proxy/spec.md
implemented: false
description: A **fix** review of `2026-09-15-initialize-after-proxy/spec.md`
fix: 2026-09-15-initialize-after-proxy/review-2.md
previous: 2026-09-15-initialize-after-proxy/review-1.md
---

# Review 2: Initialize Before Body Discovery

## Verdict

The fix is **ready for production**. The sole high-severity finding from review
#1 is resolved, review #1 contains no blocked findings to reassess, and this
iteration found no new functionality, correctness, performance, ergonomics, or
test-rigor gap.

The shared lifecycle boundary now refuses shell execution until `start` has
been recorded. Proxy adoption and loop re-entry reset that boundary, while the
executor independently rejects initialization shells and treats the refusal as
unsuppressible by `no_error`. The stabilized-read failure route therefore keeps
its non-shell catch behavior without allowing `blocked`, `failure`, or
`finalize` to execute a shell before successful preflight.

Human review is not required. The binding shell-free ruling, sequence Option A,
entry-reason behavior, and verification levels are already determined by the
specification and design, and the implementation conforms to those decisions.

## Prior Review Closure

Review #1 has a `## Findings` section rather than separate `## Unblocked
Findings` and `## Blocked Findings` sections. Its one finding was actionable
and is treated as the unblocked finding for this iteration. No blocked finding
existed, so none became newly actionable before the last implementation.

| Review #1 finding | Status in this iteration |
| --- | --- |
| [high] Staged failure handlers execute shell commands before approval | **Implemented under the superseding R2 shell-free ruling.** `LifecycleRunGuard::run_event_stack` substitutes `DisabledShellRunner` before `start`; `emit_preflight_blocked_and_finalize_in_context` and the stack-only harness route also install the disabled runner explicitly. `run_shell_action` converts the prohibition into an unsuppressible evaluation error. The new direct/proxy/loop CLI regression preserves the original plain-shell trigger and asserts two refusals, no shell markers, exact once-only `blocked`/`finalize` routing, and no provider launch with and without `-y`. The broader seven-trigger matrix covers missing dependencies, initialization errors, refused handoffs, schema/audit failures, catch evaluation errors, and harness adoption. |

## Findings

None.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| R1, AC1-AC4: initialization creates or preserves body dependencies before discovery, including guarded and unconditional includes and repeated invocation | Level 1 fake-provider CLI tests; Level 2 detached-terminal coverage for the reported proxied route | Appropriate and present. The Level 1 tests assert file contents, event order, prompt delivery, and exactly-once initialization. Level 2 verifies the visible route through a real terminal without adding semantic authority over the filesystem ordering. |
| R2, AC5: initialization and every pre-preflight catch route are shell-free regardless of approvals, dead branches, or `no_error`; later approved shells still run | Level 1 parser, executor, harness, and fake-provider CLI tests; Level 2 direct initialization-refusal capture | Appropriate and present. The review #1 trigger is covered at the real binary boundary, with positive non-shell catch and post-preflight shell controls. The new regression was also recorded as failing when both runtime backstops were temporarily disabled, so it is non-vacuous. |
| R3, AC6-AC8: stabilized rereads observe dependency/document mutation, preserve control flow, initialize once, and refuse a dependency still missing afterward | Level 1 unit and fake-provider CLI tests | Appropriate and present. These are composition, filesystem, and lifecycle-state contracts; no terminal emulator behavior is involved. |
| R4, R6, AC9: direct, inline, proxy, loop-owning, and harness-adopted entry paths share the staged ordering; sequences retain static Option A preflight | Level 1 entry-path and sequence characterization tests; Level 2 sequence diagnostic coverage | Appropriate and present. The tests distinguish staged live entry from the deliberately earlier sequence graph audit. |
| R4, AC12: lifecycle effects and later transclusions resolve one file identity across explicit-relative, repository-root, repository-scoped, implicit, and shadowed references | Level 1 fake-provider CLI and Darkmatter containment tests | Appropriate and present. Assertions avoid host-specific separator text; fresh cross-OS execution evidence remains CI/CD work and does not affect readiness. |
| R5: failures retain typed routing, execute catches once, and never launch a provider with an incomplete prompt | Level 1 typed-diagnostic and fake-provider CLI tests | Appropriate and present. The still-missing dependency test asserts the existing diagnostic and exact catch order; the shell-bearing catch variants separately assert the binding R2 refusal behavior. |
| AC10: shipped router and target preserve the reported argument shape and generated-log behavior | Level 1 passive artifact guards and hermetic end-to-end fake-provider tests; recorded Level 2 shipped-route cases | Appropriate and present. Fixture-owned spec/plan data avoids dependence on mutable repository state. |
| AC11: no-initialize, dry-run, retry, resume, and later-loop behavior remains unchanged | Level 1 unit and fake-provider CLI tests | Appropriate and present. The tests assert eager failure, absence of dry-run effects, fresh retry/resume reads, and no duplicate initialization. |

Level 3 is not required because no requirement depends on OS keyboard or mouse
injection, terminal input encoding, hotkeys, paste, or IME behavior. Level 2 is
not required for the core ordering and shell-boundary semantics; the existing
Level 2 cases appropriately cover the operator-visible terminal diagnostics.

## Verification Performed

- Read the complete specification, technical design, binding shell-free ruling,
  review #1, implementation record, acceptance evidence, production lifecycle
  boundaries, and the changed regression tests.
- Resolved the supplied review references with `biscuit_file::FileReference`.
  `@claudine/fixes/2026-09-15-initialize-after-proxy/review-1.md` resolves to
  the active prior review. The supplied
  `@prompts/_reviews/claudine/fixes/2026-09-15-initialize-after-proxy/review-1.md`
  spelling has no match, so no duplicate historical review was created.
- Bound GitNexus to this worktree at commit `faa2fc4`. Its refreshed graph
  identifies six callers of `LifecycleRunGuard::run_event_stack`, including
  the composition preflight route, harness catch protocol, lifecycle-event
  helpers, and loop catch protocol. `route_stabilized_failure` is reached from
  both staged single execution and loop execution. The final status is stale
  only for an unrelated Messenger source edit; the reviewed Claudine paths are
  indexed and were also verified directly in source.
- `just test --test compose_initialize_staged_boot`: **14/14 passed**. This
  includes the review #1 reproduction across direct, proxy, loop-owning, and
  proxied loop-owning entries, with and without `-y`.
- `just test --test compose_initialize_acceptance`: **23/23 passed**.
- `just lint`: **passed** for `claudine-catalog-types`, `claudine`,
  `claudine-contract`, `claudine-cli`, and `claudine-gen`; all eight
  `error_guards` tests passed.
- No focus-taking Level 2 or Level 3 test was run during this review. Existing
  recorded Level 2 evidence was inspected only where terminal-visible behavior
  makes that tier relevant.

## Design Assessment

The shared pre-`start` runtime boundary is the right long-term abstraction: it
protects every catch route instead of requiring each caller to remember an
approval or prohibition check. The executor-level initialization refusal and
explicit disabled runners are appropriate defense in depth, and the state
resets align with proxy and loop ownership. The added checks are constant-time
state tests on lifecycle dispatch and introduce no meaningful performance cost.
No further ergonomic or performance change is warranted.

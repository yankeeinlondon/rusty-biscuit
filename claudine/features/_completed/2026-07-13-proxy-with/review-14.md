---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T16:27:01-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-14.md
previous: 2026-07-13-proxy-with/review-13.md
next: 2026-07-13-proxy-with/review-15.md
---

# Review 14: Proxy With

## Verdict

The feature is **not ready for production**. Both review-13 findings are
closed: recursive `proxy.with` evaluation now finishes under the normal Level
1 timeout, and the authoritative architecture skill now describes the
implemented per-document launch/loop behavior. The Claudine Level 1 gate is
still red, however. Two feature-owned tests reproducibly exceed nextest's
30-second per-test timeout even when run alone, including the interpolation
conformance matrix that covers the shared syntax substrate used by
`proxy.with`.

No requirement in this feature depends on a terminal emulator's keyboard
encoder, so Level 3 is not applicable. The user-observable process and terminal
requirements continue to have Level 2 tmux coverage; no Level 2 implementation
or test changed after review 13. The readiness blocker is failing Level 1
evidence, not a missing Level 3 row.

## Findings

### 1. High: the Level 1 gate still has two feature-owned timeout failures

`just test-library` reached 853 passing tests before this review's bounded run
was stopped, but nextest had already timed out
`composition::interpolation_conformance::loop_and_lifecycle_agree_on_shared_syntax`
and `composition::looping::engine::tests::lifecycle_control::loop_initialize_stop_proceeds_into_iterations`.
Both failures reproduce when each test is run alone: two complete 30-second
attempts timed out for each test before the bounded review command stopped the
third retry.

The first test rebuilds an `EffectiveState` for every one of its 14 matrix rows
(`interpolation_conformance.rs:50-69,177-196`). Its helper does not provide a
context, so `EffectiveStateBuilder::build()` falls back to a full ambient
`ComposeContext::capture()` for every row. The feature plan still says this
test passes alone in 8.9 seconds (`plan.md:1924-1931`), but that claim is no
longer true at this HEAD.

The loop test has the same shape at a different seam. Its shared helper builds
`LifecycleRuntimeContext { context: None }`
(`looping/engine/tests/lifecycle_control.rs:52-74`), then the parity assertion
runs the loop twice (`:193-213`). Each lifecycle event therefore takes the
snapshot-less fallback instead of reusing the prepared context that canonical
production execution supplies.

Fix the test fixtures around the contract they are actually proving:

- Build one deterministic `ComposeContext` for the interpolation matrix and
  pass it through `EffectiveStateBuilder::with_context`; these cases contain no
  `ctx.*` requirement and should not perform host discovery.
- Give the loop helper one prepared context and reuse it across its lifecycle
  events. If `LifecycleRuntimeContext::context: None` remains a supported public
  execution path, cover that fallback in a separate focused test and consider
  capturing once at loop entry rather than once per event.
- Rerun the complete `just test` gate, not only the two focused rows. The full
  gate must finish without timeout retries before this criterion can close.

This is a production-readiness blocker because the normal Level 1 gate cannot
pass, and one failed row is the feature's shared loop/lifecycle interpolation
contract rather than unrelated package evidence.

### 2. Medium: the new single-capture regression test does not distinguish the bug

`nested_ctx_refs_resolve_against_one_captured_snapshot`
(`proxy_with_evaluation.rs:497-520`) claims to prove that a snapshot-less
overlay captures once, but it reads `ctx.today` twice and asserts that the two
values match. Darkmatter always includes the DateTime group even when no
`ctx.*` reference is detected (`darkmatter/.../capture/mod.rs:35-45`), and two
separate captures on the same day produce the same value. The test would
therefore pass under the pre-fix per-leaf capture behavior and does not verify
either the one-capture invariant or the union of context groups scanned from
different nested leaves.

Keep the now-passing
`nested_strings_follow_the_same_interpolation_rule` timeout regression, but add
fix-discriminating coverage for the new mechanism. At minimum, place keys from
different non-default context groups at different nesting depths and prove
both were included in the combined scan. Prefer a narrow test seam that counts
capture invocations so the central performance invariant is asserted directly
without relying on wall-clock timing.

## Review-13 closure

Review 13 is closed:

1. `nested_strings_follow_the_same_interpolation_rule` and the new nested-context
   row both pass under nextest in about 6.1 seconds, below the 30-second
   per-test ceiling. `resolve_proxy_with` now captures the snapshot-less
   fallback once and walks every overlay leaf against that context.
2. `.claude/skills/claudine/architecture.md:491-507` now states that launch
   identity and loop recognition rebuild per active document and documents the
   unowned direct-wrapper boundary. The obsolete known-gap claim is gone.

GitNexus reports the `resolve_proxy_with` change as low risk: one direct caller,
three upstream lifecycle symbols through depth three, one affected module, and
no indexed execution process.

## Verification-level audit

| User-observable requirement | Strongest current evidence | Required level | Result |
|---|---|---|---|
| Direct/proxy prompt, frontmatter, context, lifecycle order, and loop equivalence | Level 2 tmux direct/proxy matrix | Level 2 | Present |
| Provider/model/profile/binary/argv/environment/interactivity/dispatch/CWD rebuild | Level 2 fake-child launch records and pane capture | Level 2 | Present |
| MCP selection/injection and file-backed system-prompt lifetime across re-entry | Level 2 fake-child argv/file reads, with Level 1 seams | Level 2 | Present |
| Shell discovery, target policy, approval/denial, and approved bytes equal executed bytes | Level 2 target-policy and shell-byte rows | Level 2 | Present |
| Overlay precedence, schema effects, retry/resume/loop lifetime, and chain forwarding | Level 2 end-to-end rows plus Level 1 state tests | Level 2 for effects; Level 1 for pure state | Present |
| Recursive `with:` interpolation and type preservation | Level 1 focused rows | Level 1 | Present; focused rows pass |
| Shared loop/lifecycle interpolation and initialize-stop loop behavior | Level 1 conformance/loop rows | Level 1 | **Failing: both time out in isolation** |
| Typed diagnostic identity and route-equivalent rendered output | Level 2 three-route diagnostic matrix | Level 2 | Present |
| Inline closure, sequence containment, stream routing, dry-run behavior, and overlay-value non-disclosure | Level 2 composition-command/pane rows | Level 2 | Present |
| OS keyboard/modifier encoding | No such requirement | Level 3 | Not applicable |

The Level 2 matrix remains correctly Unix/tmux under the repository's ratified
platform policy: Linux CI and macOS opt-in. Windows has platform-neutral Level
1 proxy coverage but no claimed Level 2 terminal leg. No Level 2 source changed
after review 13, and Level 2 was not rerun after the Level 1 blockers
reproduced.

## Validation performed

- Read the complete feature specification, review 13, the Claudine architecture
  test-placement rules, and the current acceptance map.
- GitNexus's exact worktree index matched HEAD. Query/context traced the
  snapshot-less overlay fix, and upstream impact classified it as low risk.
- Focused nextest run: both review-13 closure rows passed in about 6.1 seconds.
- Bounded `just test-library`: 853 tests passed before interruption; nextest had
  already recorded timeout attempts for both findings above. This interrupted
  run is not counted as a passing gate.
- Each timeout test was rerun alone. Both completed two 30-second timeout
  attempts; the bounded commands stopped their third retries.
- Level 2 and lint were not rerun after the Level 1 blockers reproduced. No
  production or Level 2 source changed after review 13 besides the low-risk
  lifecycle overlay capture fix.
- The requested review/spec frontmatter was updated at the feature's canonical
  repository paths. The literal `@prompts/_reviews/claudine/features/...`
  previous-review path does not exist in this checkout; the existing canonical
  artifact is `claudine/features/2026-07-13-proxy-with/review-13.md`.

## Production readiness

`ready: false`. Make the two feature-owned Level 1 tests deterministic and
fast, strengthen the single-capture regression so it fails under per-leaf
recapture, then obtain green `just test`, `just test-l2`, and `just lint` gates
before production sign-off.

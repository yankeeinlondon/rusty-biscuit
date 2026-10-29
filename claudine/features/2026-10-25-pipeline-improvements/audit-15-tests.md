---
kind: implementation-log
---

# Test audit: 2026-10-25-loops-and-flow-control

## Changes and counts

Added **three genuine coverage test functions/Nextest identities** (gross and
net increase both three), strengthened **zero** existing tests, and modified
**zero** slow tests. No tests were split, removed, ignored, retried, or moved
between tiers. No loop engine, resolver, approval, or deadline code changed.

The three normal-CLI tests in `lifecycle_prep.rs` cover previously untested
target-classification branches and the documented unsupported boundary:

- `prep_runs_a_yaml_sequence_with_its_own_runtime_and_output_ledger` proves a
  YAML helper executes both tasks, gets the explicit overlay, initializes its
  own output ledger, finalizes before the caller resumes, and leaves the
  caller's runtime value and ledger unchanged. The helper launches no provider.
- `prep_dispatches_an_inline_prompt_before_resuming_its_caller` uses a provider
  recorder that actually updates the helper's body, as inline composition
  requires. It checks prompt order, overlay delivery, helper finalization,
  helper mutation on disk, unchanged caller source, and isolated runtime state.
- `prep_refuses_a_standalone_group_without_running_its_legs_or_continuing`
  checks the visible nested cause, caller cleanup, absence of group effects,
  and no provider launch. This proves the current restriction, not fulfillment
  of the specification's broader target contract.

An initial inline fixture failed because its provider did not update the
body. The fixture was corrected to perform that required operation; the
product's unchanged-body check was retained.

The opt-in shipped-prompt checks exposed a real authored-call defect in
`prompts/_reviews/feature-review.md`. Its finalize condition passed
`frontmatter(...) || []` to `length`. In condition syntax `||` is logical OR
and returns a boolean, so the later passive call checker correctly rejected
the document before execution. An explicit `is_array` guard now precedes
`length` on the actual property. The two existing CLI regressions failed
before this correction and passed after it; no checker or assertion was
weakened. Interpolation-mode fallback expressions elsewhere were preserved.

A stale L2 comment claiming default loop `fail_fast: true` was corrected to
the current failed-pass rule. Its test code and assertions are unchanged.

## Requirements and verification boundaries

| Criteria | Existing and added evidence |
| --- | --- |
| 1–3: uniform loops, typed results, success/cap/final-pass/break outcomes | `loop_primitives`, `looping::frame`, `looping::engine`, and `single_loop_engine` exercise the common implementation and normal CLI with independently expected prompt/result/event traces. |
| 4: loop keys and caps | `loop_config_matrix` and frame/config tests cover malformed shapes, unknown/retired keys, owner restrictions, templated caps, and override precedence. |
| 5: break propagation and cleanup | `loop_break` and `break_prep` cover nearest-loop ownership, pending concurrent/queued legs, declaration order, failure precedence, root exit codes, partial results, exactly-once cleanup, and real process interruption. |
| 6: implemented prep contracts | `lifecycle_prep`, directive/parser matrices, and runner-seam tests cover return ordering, overlays, disk-only return, cycles, cancellation, skip, child break codes, handoff, repeated calls, and failure causes; the added tests cover YAML sequence and inline dispatch. Three specification departures remain below. |
| 7–8: task guards and caller values | `task_guards` and `sequence_grammar_matrix` cover skipped/null results, no events or publication, outer-pass reevaluation, errors, invalid guarded tasks, live files versus group-start state, read-only guard scope, supplied values, and missing-value preflight. |
| 9: dry run | `sequence_dry_run` verifies a scratch Git index, flag dependency, one loop preview, no prep/lifecycle side effects, and withheld file/runtime side effects even with shell execution enabled. |
| 10: path anchors | `path_function_contracts` exercises the public expression evaluator against real isolated directories: document/repository collisions, prefixes, absolute/caller anchors, missing targets, recursive misses, outside-repository behavior, URL rejection, and typed probe failures. |
| 11: review-loop rehearsal | Explicitly deferred by the author's recorded ruling; the retired workflow was not recreated or presented as tested. |
| 12–13: shipped prompt migration | The opt-in shipped contract and route-drift tests passed, including no-provider commit behavior; the discovered feature-review guard defect was repaired. Caller-file and initialize behavior remain covered by their existing declared L1 modules; those separate modules were inspected but not rerun in this slice. |
| 14: looping-container retry | `loop_retry_passes` verifies repeated retries, earlier-pass retention, pass-entry runtime/output restoration, retry overlays, group/nested results, repeated external writes, stable count/cap, one initialize, and final-attempt finalization. |

These are internal control-flow and file-resolution contracts, so in-process
F1 assertions and normal hermetic CLI dispatch with stub providers are the
appropriate primary boundary. The tests use the real filesystem, process
execution, and cancellation paths without invoking a live agent. Terminal
ordering and routing use the existing detached tmux F2 evidence described
below. No requirement here promises native provider wire behavior or OS
keyboard encoding, so no new live-provider or L3 test was introduced. Audio
remained suppressed with fixture-private state and absence assertions.

## Recorded departures and limits

The final implementation does not satisfy every original snapshot requirement:

1. Statically known `prep` helpers are **not** inventoried transitively in the
   parent's initial shell approval. Each helper obtains its own approval when
   called, so refusal can follow earlier caller effects. No test claims the
   missing advance-inventory contract exists.
2. Standalone `kind: group` helpers are unsupported. The new negative test
   makes their current failure explicit; it does not convert that limitation
   into completed functionality.
3. There is no invocation-wide remaining deadline for helper calls. Existing
   timeout settings apply to individual provider children. Cancellation is
   exercised; it is not proof of a shared total time budget.

These are recorded implementation/product gaps from the prior review and log;
this audit did not invent a new approval or timeout architecture. The author's
explicit deferral of the retired review-loop migration is separate from those
three gaps. Canonical documentation and the mini-review own their current
status and proposed resolutions.

The added CLI cases inherit the existing `lifecycle_prep` module's Unix-only
recording-provider fixture. This audit provides macOS execution evidence, not
new native-Windows behavior coverage for those paths. Existing portable frame,
grammar, guard, and path tests remain selected where applicable. No other-OS
execution is claimed.

## Verification

All L1 commands used the canonical Nextest-backed recipes with parallel
scheduling. From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(loop_primitives::) | test(loop_retry_passes::) | test(loop_break::) | test(lifecycle_prep::) | test(task_guards::) | test(sequence_dry_run::) | test(loop_config_matrix::) | test(loop_gate_ambient::) | test(composition::looping::) | test(break_prep::) | test(single_loop_engine::) | test(lifecycle_directive_matrix::) | test(sequence_grammar_matrix::)' just test
```

**625/625 passed**, maximum **4.176 seconds**, total execution 25.844 seconds.
The new inline, YAML sequence, and unsupported-group tests took **2.995**,
**2.503**, and **0.969 seconds** respectively in that parallel run. Log:
`/tmp/pipeline-s15-final.log`. The initial narrower run passed 361 tests,
maximum 3.550 seconds; it adds no distinct identities beyond the final run.

From `darkmatter/`:

```sh
BISCUIT_TEST_FILTER='test(path_function_contracts::) | test(path_components::) | test(file_index::)' just test
```

**9/9 passed**, maximum **0.124 seconds**. The Windows-native-path case is
conditionally compiled on Windows and was not run on this macOS host. Log:
`/tmp/pipeline-s15-paths.log`.

From `claudine/`, opt-in live internal prompt checks:

```sh
BISCUIT_TEST_FILTER='test(shipped_prompt_contract::) | test(shipped_prompt_route_drift::)' just test-prompts
```

**30/30 passed**, maximum **3.222 seconds**, after the authored-call fix.
Log: `/tmp/pipeline-s15-prompts.log`. Before that fix the two actual-CLI
feature-review regressions failed with the located incompatible-call error.

Total new execution evidence: **664 distinct L1/prompt identities passed**,
zero over five seconds. Reused L2 evidence: the preceding audit's canonical
parallel run passed **292 tests**, maximum **4.997 seconds**, including loop
gate order, per-pass finalization, cancellation, and looping-target routing.
Its log is `/tmp/pipeline-s14-l2.log`. No L2 behavior code, assertions, or
fixtures changed in this slice, so that passing evidence was not rerun. The
comment correction does not affect it; the changed feature-review prompt is
not a source read by the selected loop tests.

`git diff --check` passed. No full-area suite, formatting command, or separate
lint gate was run; the coordinator owns the final broader checks.

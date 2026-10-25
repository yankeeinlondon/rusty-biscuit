---
kind: implementation-log
---

# Test audit: 2026-10-22-tts-prewarm

Added **2 genuine coverage test functions / Nextest identities** (gross and
net **+2**); **0 scheduling-only identities**, **0 existing tests strengthened**,
and **0 slow tests modified**. The final scoped runs passed **218 distinct
L1 tests**, with a maximum of **2.450 seconds**. All audio observations were
silent. No detached-playback, terminal, or browser boundary changed, so L2
was not applicable.

## Defect found and repaired

Later container support had not been connected to the original pre-warm
collector. Sequence preflight descended into container children but skipped
the root, group, and nested-sequence lifecycle stacks. Container execution
also had no submission site for its captured environment or refreshed
frontmatter. Consequently even literal container phrases could remain cold.

The new library regression first failed with **1 queued candidate instead of
5**: only the prompt document was collected. Preflight now walks the parsed
root and nested container lifecycles, preserving the enclosing frontmatter
for inline containers and the referenced sequence's own frontmatter. It
still performs no composition, reference resolution, guard evaluation, or
side effects. Caller-supplied keys remain late in every lexical scope.

At container bootstrap and `start`, the existing `collect_and_submit` entry
point now receives the actual frontmatter and captured context used by the
events. `TaskExecution::prewarm_container` covers nested groups and sequences;
`RootRun::event` covers the root. The handle retains caller/sequence-injected
keys, and candidates use the same user settings, worker, gates, and
invocation-wide de-duplication as prompt speech. Publication is unchanged.

The second new test crosses the real Claudine CLI boundary. A nested
root → group → inline sequence → referenced sequence fixture declares only
never-firing failure speech. Its provider stub waits for four recorded,
resolved `env` phrases before recording that it launched. The test asserts
that observation, the exact phrase set, one provider launch, and no audio
spool. It would fail if the values appeared only after child completion.

## Requirement coverage

| Requirements | Evidence and boundary |
| --- | --- |
| Structural collection, bundles, aliases, named/positional text, nested branches, loop gate; exclude initialize | Existing `collect::tests` assert authored paths, order, scopes, and exact phrase sets. New container graph test covers all current container owners and lexical frontmatter. Internal transformation contract, F1. |
| Early/late values, newly declared globals, bracket access, task overlays, speculative safety | `classify::tests` cover every reserved root, supplied captures, unresolved values, computed reads, operators, calls in untaken branches, shell syntax, single evaluation, errors, and empty text. New container test proves caller keys stay late across lexical scopes. F1. |
| Static sequence preflight and proxy/retry/reread wiring | Library graph tests plus real CLI recorder tests prove all three prompts are submitted before step one, provider launch survives blocked preparation, proxy overlays apply, retry text changes, and initialization rereads discover new candidates. New CLI container test proves captured values arrive before child launch. Real process wiring with synthetic provider/preparer, F1. |
| Configuration compatibility, validation, playback config, TUI preservation, off switches | `config::tts`, loader robustness/shape/round-trip tests, hook/lifecycle settings tests, TUI tab/reducer tests, worker gates, and CLI dry-run/config tests. Per-candidate config and de-duplication include all config fields. F1. |
| Nonblocking serial worker, failure policy, sanitized warning, no retry, cancellation without draining | Controllable fake futures observe calls, completion, seriality, cancellation, shared handles, and one sanitized warning; actual CLI stub proves provider launch proceeds. F1 owned concurrency contract, with real threads/processes. |
| No audio/cache/spool under dry run; ordinary publication while pre-warm blocks | Frozen shipped prompt under isolated temp/cache/spool directories, accepted spelling tests, and existing `audio_emission` publication test. These observe the real local publication boundary without playing audio. Native synthesis/cache behavior belongs to the biscuit-speaks dependency and was separately audited in step 9. |
| Integrated container execution, recovery, and guard invariants | Existing root/nested/group container contracts, loop and retry CLI cases, lifecycle ownership guards, context-construction guards, and fixture-seam guards passed after the fix. |

The current authored grammar rejects retired task `setup`/`teardown` keys;
CLI rejection cases remain, while explicit internal models retain legacy
runtime coverage. The audit does not restore obsolete authoring syntax.

## Verification

From `claudine/`, through canonical Nextest-backed recipes:

```sh
just test-library container_events_are_collected

BISCUIT_TEST_FILTER='test(prewarm) | test(config::tts::) | test(tts_config_input_robustness_matrix) | test(tts_shapes_bridge) | test(tts_object_round_trips) | test(provider_less_object) | test(tts_settings) | test(commands::config_tui::tabs::tts::) | test(commands::config_tui::reducers::tests::provider_selection) | test(lifecycle_one_implementation_guard) | test(context_construction_guard) | test(test_seam_gate_guard) | test(sequence::task::tests::container) | test(sequence_containers) | test(container_loops)' just test

BISCUIT_TEST_FILTER='test(sequence::task::tests::nested_sequences) | test(sequence::task::tests::group_preparation_contract) | test(sequence::task::tests::container_regressions) | test(loop_primitives) | test(loop_retry_passes)' just test
```

The first command is the observed red-before run. The final two parallel L1
runs passed **177** and **54** observations respectively, overlapping by
13 existing container tests: **218 distinct tests**. Their maxima were
**2.450 seconds** and **1.786 seconds**. The new tests took **0.035 seconds**
and **1.066 seconds**. Logs: `/tmp/pipeline-s12-red.log`,
`/tmp/pipeline-s12-consumers.log`, and
`/tmp/pipeline-s12-container-consumers.log`.

An earlier baseline passed 100 tests (maximum 2.659 seconds), and an
intermediate fixed run passed 102 (maximum 2.822 seconds). During the fix,
one build used a nonexistent immutable guard accessor; it was corrected to
the existing `guard_mut` accessor before the final runs. Compilation is
separate from test duration.

Local execution evidence is macOS. Portable library cases use temporary
directories and injected preparers; the existing CLI suite uses its Unix
shell-script provider fixture. No platform-specific implementation changed.
No cross-OS run or full-area gate is claimed by this slice.

Reviewed source comments for the behavior change, updated the collector's
container scope, and removed the obsolete claim that `prep` is a future
directive. Documentation and skill updates belong to the concurrent docs
agent. `git diff --check` passed. From the repository root,
`cargo clippy -p claudine -p claudine-cli --all-targets --features claudine-cli/test-fixtures -- -D warnings`
passed (exit 0). The lint run followed all timing runs, so it did not compete
with test execution.

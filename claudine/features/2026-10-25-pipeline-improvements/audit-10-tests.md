---
kind: implementation-log
---

# Test audit: 2026-10-20-lifecycle-grammar

Added **5 test identities**, all net additions: **4 new coverage identities**
and **1 scheduling identity**. Modified **1 original slow L2 test**; no slow
L1 tests were found. No production behavior changed.

Final verification passed **246 distinct L1 tests** and **13 distinct L2
tests**. The L1 maximum was **3.213 seconds**; the L2 maximum was **4.791
seconds**. Five of the L1 tests belong to the opt-in live-prompt binary.

## Coverage changes

The new parser test rejects retired grammar in unselected, nested bodies and
checks the exact authored path and removed spelling. Its two cases reach a
second-level `else` dictionary and an `else` list inside `loop.gate`, rather
than repeating the existing root and first-level `then` cases.

A new removed-grammar table row authors a retired communication `route`
parameter in an unselected `else`. The existing macro generates one test each
for `compose`, `inline-compose`, and `sequence`. All three require the typed
diagnostic, nested authored path, replacement guidance, exit status 1, and an
absent provider-launch marker. The existing roster guard covers the new row.
Together with the parser test, these are **4 genuinely new coverage tests**.

The original shipped-router equivalence test took **9.526 seconds** because
it executed both complete three-phase terminal runs sequentially. The direct
and routed runs now have separate Nextest identities, and each asserts the
same exact three-phase trace and exactly three provider launches. Equality
with the same canonical trace preserves the original route equivalence check,
including the real shipped router, drift-guarded target, and all phases. The
original routed test identity remains, while the direct identity is new:
**1 extra scheduling identity**, no removed identities. Both cases finished
below five seconds in the parallel slice; the maximum was **4.791 seconds**.

## Requirement coverage

| Requirements | Evidence |
| --- | --- |
| AC1–4, AC17: lists, canonical dictionaries, conditionals, depth, ordering, flags, aliases, shape errors, branch validation | `composition::lifecycle::tests::grammar` plus executor `conditions_control`. Existing tests assert parsed paths/order, depth-five acceptance/depth-six rejection, selected-only execution, and nested controls. New tests cover unselected nested removed grammar. |
| AC5: retry/proxy overlays and transition-only lifetime | `retry_with_evaluation`, `overlay_layering`, and real CLI `compose_retry_overlay`; native values, atomic failure, repeated retry, retry/resume/proxy transitions, fresh rereads, and unchanged disk bytes. |
| AC6–7: removed spellings and loop bindings | Parser grammar tests, all three CLI entry modes, `loop_gate_ambient`, `gate_ambient`, `removed_loop_reads`, and Darkmatter host-global composition tests. Literal old names remain data; authored expression reads fail before launch or at late rendering. |
| AC8: authored audio order and survival across transitions | `audio_emission`: fake-emitter order tests plus real private queue publication through `LockedAudioSpool`, selected branches, durable records across retry, and dry-run nonpublication. Tests remain silent. |
| AC9: shared grammar ownership | `lifecycle_lift_ready_guard` and `lifecycle_one_implementation_guard`. Later pipeline work moved the parser into Darkmatter; the audit preserves that final ownership rather than reinstating the snapshot's temporary location. |
| AC10–11: shipped and frozen artifacts | Opt-in lifecycle corpus and all four route-drift guards passed without updating hashes. The CLI selected-branch tests and L2 shipped-router/optional-message cases exercise normal invocation. |
| AC12: editor paths and schema integration | DMLS nested-span tests, mapping-only corpus, and host-global composition tests. Current authoritative host schemas replace the original interim extension; no obsolete extension was restored or corpus re-blessed. |
| AC13–16: docs, skills, migration | Concurrent documentation audit owns current docs and canonical references. Parser rejection and shipped-corpus tests prove executable grammar; no new CI cells or runner configuration were introduced. Historical spec text remains a snapshot. |

The grammar transformation is a synthetic-input contract and is best checked
in-process. Real CLI tests prove entry-point wiring and lack of provider
launch on rejection. The tmux tests add real terminal observation of authored
communication order, loop gates, selected commit branches, and shipped route
behavior. Fake providers intentionally isolate grammar and lifecycle behavior;
these tests make no claim about a provider's native protocol. Queue tests
observe actual durable publication without allowing playback. No browser or
terminal focus was taken.

## Verification

From `claudine/`, parallel L1:

```sh
BISCUIT_TEST_FILTER='test(composition::lifecycle::tests::grammar::) | test(conditions_control::) | test(retry_with_evaluation::) | test(overlay_layering::) | test(audio_emission::) | test(compose_removed_grammar::) | test(compose_retry_overlay::) | test(loop_gate_ambient::) | test(lifecycle_set_shell_values::) | test(lifecycle_lift_ready_guard::) | test(gate_ambient::) | test(removed_loop_reads::) | test(lifecycle_one_implementation_guard::)' just test
BISCUIT_TEST_FILTER='test(internal_lifecycle_corpus_parses_under_current_grammar) | test(shipped_prompt_route_drift::)' just test-prompts
```

The final ordinary slice passed **215 tests**, maximum **3.213 seconds**;
the opt-in slice passed **5 tests**, maximum **0.120 seconds**. The initial
ordinary run passed 211 tests before the four additions.

From `darkmatter/`, parallel L1:

```sh
BISCUIT_TEST_FILTER='test(nested_span_tests::) | test(mapping_only_corpus::) | test(test_mapping_items_descend_to_their_keys) | test(host_globals_compose::)' just test
```

Passed **26 tests**, maximum **2.878 seconds**. Logs:
`/tmp/pipeline-s10-l1-final.log`, `/tmp/pipeline-s10-dmls.log`, and
`/tmp/pipeline-s10-prompts.log`.

Only after all L1 runs finished, from `claudine/`, parallel detached tmux L2:

```sh
just test-l2 -p claudine-cli --features prompt-tests level2_lifecycle_success_communication level2_lifecycle_blocked_communication level2_lifecycle_success_error_downgrades level2_lifecycle_loop_ level2_lifecycle_shipped_implement_ level2_shipped_implement_plan_
```

The canonical recipe completed successfully. It ran the **13 distinct tests**
twice, with all **26 observations passing**, maximum **4.791 seconds**.
The explicit package selector is necessary because this area recipe forwards
arguments to its generator leg too, and the generator has no `prompt-tests`
feature. Earlier attempts without that selector passed the CLI tests but
failed the generator feature check; one initial overly narrow name selected
no CLI tests. These attempts are not claimed as passing recipe evidence.
The final command includes the owning CLI in both recipe legs, so the second
leg repeats its tests; duplicates are not counted as added coverage or distinct
tests. Log: `/tmp/pipeline-s10-l2-verified.log`.

`git diff --check` passed. No full-area, lint, or cross-OS run is claimed by
this test-only slice; the coordinator owns the final package gates. No tests
were ignored, re-tiered, retried, or stripped of cases to meet the time limit.

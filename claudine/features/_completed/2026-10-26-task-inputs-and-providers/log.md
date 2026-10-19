---
feature: 2026-10-26-task-inputs-and-providers
implementation_log: implementation-log.md
implementation_1: "2026-10-18T06:40:28-07:00"
implementation_2: "2026-10-18T07:03:02-07:00"
implementation_3: "2026-10-18T08:19:41-07:00"
---

# Review Repair Log: Task Inputs And Providers

The implementation itself is logged in `implementation-log.md`. This file
records the review-to-implement cycles.

## Implementation of Review Findings #1

> **started at:** 2026-10-18T06:40:28-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- inventory of `review-1.md`:
        - `findings: []` and `observations: []`: the review stopped at its first-cycle decision check, so it has no code findings
        - `human_in_the_loop` holds three schema tradeoffs (HITL-1 to HITL-3), and Ken Snyder ruled on all three under "Human rulings" on 2026-10-18
        - the only actionable work is to confirm each ruling's stated prerequisites (documentation, follow-up spec, schema shapes) and fix any that are missing
        - "Other implementation decisions" lists items deferred to the later code review (retry-model concern, stub provider substitution, cache warming); the review says these need no ruling, so they are carried as nonblocking rows rather than repaired here
        - `implemented` was already `false` before repairs started

### Finding Ledger

| ID         | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ---------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-HITL-1  | Narrower editor checks inside the three typed levels (ruling 1: accepted as a temporary limit) | already fixed | every child verified, no repair needed |
| R1-HITL-1.1 | `cancelled` typed on levels 1–2 only, `loop` type on level 1 only, other container events shape-only | already fixed | `claudine/schemas/sequence-types.yaml` lines 51/57/69/75 (level 1), 100/106/118/124 (level 2: `loop` is `object`, `cancelled` is `cancelled-stack`), 151/157/169/175 (level 3: `cancelled` is `loose-body`) |
| R1-HITL-1.2 | `shell`, `side_effect`, `task` stay `unknown`, `prompt` stays `string`, `step` keeps number/boolean arms | already fixed | `sequence-types.yaml` lines 31–35 (step arms), 40–43, 89–92, 138–141 (task fields at each level) |
| R1-HITL-1.3 | `docs/topics/schemas.md` (Sequence containers) documents where editor checks stop | already fixed | `schemas.md` "Sequence containers" per-level table and "What only the parser checks" bullet on container events below the typed levels |
| R1-HITL-1.4 | The limit has an end: 2026-10-30-claudine-recursive-schemas commits to typing every container event at every depth | already fixed | that spec, section "Containers recurse, and every container event is typed" |
| R1-HITL-2  | One declaration per key (ruling 2: accepted, with follow-up) | already fixed | every child verified |
| R1-HITL-2.1 | `sequence.yaml` alone declares `sequence`/`template`/`fail_fast`; `claudine.yaml` declares root events and `loop` | already fixed | `claudine/schemas/sequence.yaml` payload has exactly those three keys; `schemas.md` "When the schemas apply" states the split |
| R1-HITL-2.2 | Late event-key completion documented under "In the editor" | already fixed | paragraph present in the working tree of `schemas.md` ("Each key is declared by exactly one trigger …") |
| R1-HITL-2.3 | Follow-up proposal `cross-trigger-completion` exists in `_unscheduled` | already fixed | `claudine/features/_unscheduled/cross-trigger-completion.md`, related to this feature |
| R1-HITL-3  | Values the schema accepts but the parser rejects (ruling 3: accepted departure from AC9) | fixed | every child verified; the docs gap in 3.2 and the sibling 3.4 repaired with tests |
| R1-HITL-3.1 | `sequence: 42` passes the schema, the parser rejects it, and the docs say so | already fixed | `schemas.md` "When the schemas apply" table row for `sequence: 42`; `sequence.yaml` header comment |
| R1-HITL-3.2 | root `fail_fast: null` passes the schema, the parser rejects it, and the docs say so | fixed | the ruling said the docs already covered it, but they did not; added the row to `schemas.md` "When the schemas apply"; new `root-fail-fast-null` case in `lib/tests/l1/sequence_schema.rs::removed_and_misspelled_keys_are_reported_by_name` asserts schema-valid and parser rejection naming `fail_fast` |
| R1-HITL-3.4 | sibling: `sequence: true` (native boolean) behaves like `sequence: 42` and is documented and tested | fixed | `schemas.md` row now reads `sequence: 42` or `sequence: true`; new `sequence-boolean` case in `sequence_trigger_activation_follows_the_value_shape` asserts schema-valid and `CompositionError::SequenceInvalid` from the parser |
| R1-HITL-3.3 | group-key errors reported at `/…/group` with the offending key in the message, and documented | already fixed | `schemas.md` "Sequence containers": "the problem is reported at the group (`/sequence/1/group`) with the group's value" |
| R1-NB-1    | Nonblocking: retry-model concern, stub-provider substitution, model-cache warming, static/runtime task diagnostic paths | deferred | the review assigns these to the later code review, not to this cycle; no ruling or repair requested |

- starting the work on 'R1-HITL-1 and R1-HITL-2 verification' at 06:42:10
        - checked the schema files, `schemas.md`, the recursive-schemas spec, and the follow-up proposal against the ruling text; all match
- work completed for 'R1-HITL-1 and R1-HITL-2 verification' at 06:44:30
- starting the work on 'R1-HITL-3' at 06:44:40
        - defect class: a value the schema accepts and only the parser rejects is a contract the docs must state, and each such value must be listed
        - sibling sweep over the values in `sequence.yaml`'s payload and match: `sequence` number (documented), `sequence` boolean (coerced like a number per the `sequence.yaml` comment, not listed in the docs), root `fail_fast: null` (not listed in the docs; the "When the schemas apply" table only lists `fail_fast: true`)
        - parser rejection of root `fail_fast: null` is already tested: `lib/src/composition/sequence/tests.rs` case "root fail_fast null" (expects an error, Markdown and YAML sources); the parser checks key presence (`composition/sequence/mod.rs:123`, `formal.rs:64`)
        - repaired the docs: `docs/topics/schemas.md` "When the schemas apply" table now lists `sequence: true` beside `sequence: 42` and adds a `fail_fast: null` row
        - added tests for both, as inline cases so the fixture-name guard is unchanged: `sequence_schema.rs` `sequence-boolean` and `root-fail-fast-null`, each asserting the schema verdict (valid) and the parser's rejection
        - `cargo nextest run -p claudine -E 'test(/sequence_schema::/)'`: 7 passed
        - repaired directly by the orchestrator, without a subagent: the change is two table rows and two test cases
- work completed for 'R1-HITL-3' at 06:45:30

### Completion Report

- outcome: **complete**; elapsed about 8 minutes (06:40:28 to 06:48:47)
- parent findings: total 3 = fixed 1 (R1-HITL-3) + already fixed 2 (R1-HITL-1, R1-HITL-2) + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - the review's `findings` and `observations` lists are empty; the three parents are the human-in-the-loop items, all ruled on by the author on 2026-10-18
        - the nonblocking row R1-NB-1 (items the review assigns to the later code review) is not a parent finding and stays with that review
- verified repairs:
        - R1-HITL-3.2: ruling 3 said `schemas.md` already documented root `fail_fast: null`, but it did not; the row is now in the table and covered by a test
        - R1-HITL-3.4, found by the sibling sweep: native boolean `sequence: true` is coerced like `sequence: 42`; it is now documented and tested
- open finding or child IDs: none
- no cross-OS check: the change is one doc table and two in-process test cases, with no paths, processes, or `cfg` code
- changed files:
        - `claudine/docs/topics/schemas.md`: "When the schemas apply" table (R1-HITL-3.2, R1-HITL-3.4)
        - `claudine/lib/tests/l1/sequence_schema.rs`: `sequence-boolean` and `root-fail-fast-null` cases (R1-HITL-3.2, R1-HITL-3.4)
- final verification:
        - `cargo nextest run -p claudine -E 'test(/sequence_schema::/)'`: 7 passed (R1-HITL-3)
        - `just lint` in `claudine/`: exit 0
        - `just test` in `claudine/`: 10375 run, 10375 passed, 9 skipped, exit 0

## Implementation of Review Findings #2

> **started at:** 2026-10-18T07:03:02-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- inventory of `review-2.md`:
        - four findings (R2-F1 to R2-F4), all unblocked; no blocked findings, no recurrence
        - three observations (removed task models as planned fallbacks, retry composition environment, looping prompt with earlier-output hint) are explicitly nonblocking and are not repaired here
        - `implemented` was already `false` before repairs started

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1    | Prompt composition retains an earlier launch identity | fixed | all 14 children verified; `identity_rows!` in `cli/tests/l1/sequence_task_providers.rs`; full `just test` green |
| R2-F1.1  | Dynamic `with.agent` from earlier shell output: prompt and child both say Goose | fixed | `identity_dynamic_with_agent` (body, child `AGENT`, completion `via Goose`); fails with selector disabled |
| R2-F1.2  | Dynamic document `agent` from `seq.outputs[0]`: both say Goose | fixed | `identity_dynamic_document_agent`; fails with selector disabled |
| R2-F1.3  | Dynamic `with.model`: body renders `new-model` | fixed | `identity_dynamic_with_model`: `BODY_MODEL=new-model`, env `claude|new-model`, `--model new-model`; fails under either mutation |
| R2-F1.4  | Concurrent group leg `with.agent: '{{ group.choice }}'`: both say Goose | fixed | `identity_group_leg_agent`; fails with selector disabled |
| R2-F1.5  | Fresh task document switching agent: prompt, child, completion agree on Goose | fixed | `identity_rewritten_document_agent` (prompt, child, completion Goose); fails under either mutation |
| R2-F1.6  | Static own-provider control | clean | `identity_static_own_agent` passes with and without the fix |
| R2-F1.7  | External task static control | clean | `identity_external_task_static_agent` |
| R2-F1.8  | Looping prompt static control | clean | `identity_looping_prompt_static_agent` |
| R2-F1.9  | Proxy target control | clean | `identity_proxy_target`: each body names its own provider |
| R2-F1.10 | Body-step path stays consistent | clean | `identity_body_step`; `jit.rs` `step_env_overrides` path unchanged |
| R2-F1.11 | Inline nested sequence dynamic hint: both say Goose | fixed | `identity_inline_nested_dynamic_agent` |
| R2-F1.12 | Dynamic task with no root fallback: prompt says Goose | fixed | `identity_dynamic_with_agent_without_fallback` (also completion) |
| R2-F1.13 | External task with dynamic hint: both say Goose | fixed | `identity_external_task_dynamic_agent` |
| R2-F1.14 | File-reference nested sequence dynamic hint: both say Goose | fixed | `identity_file_nested_dynamic_agent` |
| R2-F2    | Explicit selection flags do not override circular hints | fixed | precedence applied per key in `select_one`; all children verified; mutation fails 10 row tests |
| R2-F2.1  | Document circular `agent` + `--claude` selects Claude | fixed | `a_provider_flag_replaces_a_circular_document_agent`, `a_provider_option_replaces_a_circular_document_agent`, unit `a_provider_flag_makes_a_circular_agent_irrelevant` |
| R2-F2.2  | `with.agent` circular + `--claude` selects Claude | fixed | `a_provider_flag_replaces_a_circular_with_agent` |
| R2-F2.3  | Document circular `model` + `--model explicit-model` | fixed | `an_explicit_model_replaces_a_circular_document_model` (argv `--model explicit-model`) |
| R2-F2.4  | Caller setter agent control | clean | `a_caller_agent_replaces_a_circular_document_agent` |
| R2-F2.5  | Caller setter model control | clean | `a_caller_model_replaces_a_circular_document_model` |
| R2-F2.6  | Noncircular dynamic agent + `--claude` control | clean | `a_provider_flag_replaces_an_agent_decided_at_launch` |
| R2-F2.7  | Circular hint with no override still rejected | clean | `a_hint_reading_its_own_identity_fails_as_circular`, `a_circular_agent_with_no_override_fails_before_any_launch` |
| R2-F2.8  | `with.model` circular + `--model` | fixed | `an_explicit_model_replaces_a_circular_with_model` |
| R2-F2.9  | Captured `MODEL` env outranks circular document model | fixed | captured `MODEL` and `CLAUDE_MODEL` CLI cells; unit `a_higher_model_source_makes_a_circular_model_irrelevant` |
| R2-F2.10 | Circular `model` on a task whose provider is decided at launch, with only a provider-specific model variable set (discovered) | clean | by contract: spec §5 excuses a lower hint only for a statically known higher-precedence override; documented on `model_decided_above_hints` |
| R2-F3    | Static errors in selection inputs are postponed until execution | fixed | all children verified; mutation (fix reverted) launches task 1 |
| R2-F3.1  | `with.agent` unknown function fails preflight, no launch | fixed | lib `a_with_selection_input_that_fails_to_evaluate_is_a_failure`; CLI `an_invalid_with_selection_expression_fails_before_any_task_launches` (zero launches) |
| R2-F3.2  | `with.model` unknown function fails preflight, no launch | fixed | same two tests, `model` iteration |
| R2-F3.3  | Document `agent` invalid expression control | clean | `an_invalid_document_selection_expression_fails_before_any_task_launches`, `an_otherwise_static_hint_that_fails_to_evaluate_is_a_failure` |
| R2-F3.4  | Document `model` invalid expression control | clean | same CLI test, `model` iteration |
| R2-F3.5  | Genuine late values remain deferred | clean | `late_with_selection_inputs_remain_decided_at_launch` plus existing earlier-output, runtime-write, group-snapshot CLI tests |
| R2-F3.6  | CLI gate receives `Failed` for static failures | clean | `own_hints` maps `Failed` to a located `SequenceTaskProviderSelection`; CLI test above proves wiring |
| R2-F3.7  | Hint reading a key only a runtime `set` creates is decided at launch, not `Static null` (discovered) | fixed | `dependencies` checked with `derived_deferred_keys`; lib `a_hint_reading_a_key_only_a_runtime_write_creates_is_decided_at_launch`, CLI `a_task_agent_read_from_a_key_only_a_runtime_write_creates_is_decided_at_launch` |
| R2-F4    | Empty provider hints bypass the canonical fallback rule | fixed | category normalized at the source (`ProviderPrediction::class`); .2/.4 expected result disputed against the shipped schema and repaired to an early located failure instead |
| R2-F4.1  | Frontmatter `agent: null` inherits enclosing Claude | fixed | `empty_hint_frontmatter_null_inherits`; lib `an_empty_agent_hint_is_missing_a_provider_or_a_located_failure` |
| R2-F4.2  | Frontmatter `agent: []` inherits enclosing Claude | disputed | "inherit Claude" contradicts `claudine.yaml` `agent: string[](min(1))` and `docs/topics/schemas.md` ("`agent: []` … invalid"); repaired to a located preflight failure with zero launches: `empty_hint_frontmatter_list_fails_preflight_before_any_task_launches` |
| R2-F4.3  | `with.agent: null` inherits | fixed | `empty_hint_with_null_inherits` |
| R2-F4.4  | `with.agent: []` inherits | disputed | same schema authority as R2-F4.2; `empty_hint_with_list_fails_preflight_before_any_task_launches` |
| R2-F4.5  | Caller setter `agent=null`/`[]` uses missing-provider gate | fixed | `empty_hint_setter_null_uses_the_missing_provider_gate`, `empty_hint_setter_list_uses_the_missing_provider_gate` (`AgentResolutionFailed`, zero launches) |
| R2-F4.6  | Absent field control | clean | `empty_hint_absent_inherits` |
| R2-F4.7  | Nonempty scalar/list controls | clean | `nonempty_hint_scalar_launches_its_provider`, `nonempty_hint_list_launches_its_one_installed_member` |
| R2-F4.8  | Body/enclosing hint invariant | clean | body/enclosing checks use parsed `EffectiveSelectionHints::names_agent` (`cli/.../sequence/providers.rs`) |
| R2-F4.9  | Review projection: empty hints inherit or produce editable row | fixed | `review::tests::a_null_agent_hint_is_missing_a_provider_and_reviewable`, `an_empty_agent_list_fails_selection_where_it_was_written` |
| R2-F4.10 | Consumer: `launch_trigger_schemas` step-document test used `agent: []` as its launch-schema-only control (discovered) | fixed | switched to `interactive: sometimes`; `a_sequence_step_document_with_an_invalid_control_fails_before_its_provider` passes |

### Progress

- starting the work on 'R2-F3 static errors in selection inputs postponed' at 07:03:51 (delegated to a foreground subagent)
        - defect class: a failed early evaluation of a static input was recorded as a late dependency, discarding its cause, so an authoring error surfaced only at the task's turn after earlier tasks launched
        - preflight's `with` reader now returns `PreparedWith { evaluated, deferred, failed }`; ordinary evaluation errors keep their `LifecycleCause` in `failed` (still listed in `deferred`, so shell discovery and the runtime path are unchanged)
        - provider prediction turns a failed `with.agent`/`with.model`, or a document hint reading a failed `with` entry, into a located `HintState::Failed`; genuine late reads and circular `env.AGENT`/`env.MODEL` are still detected first by the existing `classify_reads`
        - sibling sites checked: `preflight/mod.rs` context-construction `Err(_)` arm (not an expression error, documented), `providers.rs` `None => DecidedAtLaunch` (now only reachable from that arm), `defer_runtime_written` (a runtime write genuinely outranks), `task/mod.rs` runtime `deferred_with` consumer (unchanged on purpose), `owner_schema .ok()` and CLI `hints_from_map(...).ok()` (not selection decisions / accepted rule): clean
        - tests: library `a_with_selection_input_that_fails_to_evaluate_is_a_failure`, `a_document_hint_reading_a_failed_with_entry_is_that_entrys_failure`, `a_failed_with_entry_no_hint_reads_leaves_the_prediction_alone`, `late_with_selection_inputs_remain_decided_at_launch`; CLI `an_invalid_with_selection_expression_fails_before_any_task_launches`, `an_invalid_document_selection_expression_fails_before_any_task_launches`; mutation-checked (fix reverted → task 1 launches)
        - docs: `docs/cli/sequence.md`, `docs/topics/flow-control/sequences.md`, `docs/topics/execution-flow.md`
        - discovered R2-F3.7 (pre-existing): a hint reading a key only a runtime `set` creates is predicted `Static null` rather than decided at launch, because `reads` records only keys present in the early state; handed to the R2-F4 repair since it feeds the same category decision
        - checks: `just test-library providers::tests` 16/16, `just test-library preflight` 237/237, `just test-cli sequence_task_providers::` 13/13, `with_file_provenance::` 15/15, `sequence_errors_cli::` 52/52, `just lint` pass
- work completed for 'R2-F3 static errors in selection inputs postponed' at 07:13:51
- starting the work on 'R2-F4 empty provider hints bypass fallback' (plus R2-F3.7) at 07:13:51 (delegated to a foreground subagent)
        - defect classes: (1) raw field presence picked the provider category after the canonical parser had normalized the value to no hint; (2, R2-F3.7) a hint only counted as reading a key that had an early value, so a key created only by a later runtime write was invisible to the runtime-write deferral
        - `ProviderPrediction::class` now reads the static `agent` through the canonical parser (new `agent_value_names_a_hint` in `lib/src/composition/hints.rs`), so `null` (any layer) and a caller setter's `[]` are `Missing`; `""` stays an invalid name; CLI "names an agent" checks (`providers.rs`, `task_run.rs`) now share `EffectiveSelectionHints::names_agent`
        - R2-F3.7: `ProviderPrediction.reads` became `dependencies` (hint value plus every value it reads) and is checked with Darkmatter's `derived_deferred_keys` against the written keys, so an unset key written later is decided at launch
        - conflict found: the shipped launch schema declares an agent list `string[](min(1))` and `docs/topics/schemas.md` documents `agent: []` as invalid, so a document or `with` `agent: []` cannot inherit as the review expected; after the first repair it passed selection and failed the launch schema at task start, after earlier tasks ran
                - orchestrator decision: keep the schema (owned by a depended-on spec) and apply its existing rule early: a static document or `with` `agent: []` is a located preflight failure (`HintState::EmptyAgentList`, problem `SequenceTaskProviderProblem::EmptyAgentList`, "an `agent` list needs at least one entry"), and nothing launches
                - sibling check: root sequence `agent: []` already fails preflight (root schema check or missing-provider gate); a referenced nested sequence file with `agent: []` and a body step slipped through and now fails selection at that file's `agent` (`OwnHints::BodyEmptyAgentList`) unless a caller `agent=` setter replaces it
                - first follow-up reused the `Evaluation` problem ("could not be evaluated: …"); the orchestrator replaced it with the dedicated variant so the diagnostic is accurate, and the CLI test now asserts the misleading wording is absent
        - review projection: `review::tests::a_null_agent_hint_is_missing_a_provider_and_reviewable` (null inherits or gives editable `Unresolved{NoAgent}` rows), `an_empty_agent_list_fails_selection_where_it_was_written`
        - CLI cells (`selection_cells!`, one process per cell): `empty_hint_frontmatter_null_inherits`, `empty_hint_with_null_inherits`, `empty_hint_setter_{null,list}_uses_the_missing_provider_gate`, `empty_hint_absent_inherits`, `empty_hint_empty_string_is_an_invalid_name`, `empty_hint_model_null_and_list_are_no_model`, `nonempty_hint_scalar_launches_its_provider`, `nonempty_hint_list_launches_its_one_installed_member`, `empty_hint_{frontmatter,with}_list_fails_preflight_before_any_task_launches`, `empty_hint_nested_sequence_list_fails_its_body_step_before_any_task_launches`, `a_task_agent_read_from_a_key_only_a_runtime_write_creates_is_decided_at_launch`
        - library: `an_empty_agent_hint_is_missing_a_provider_or_a_located_failure`, `a_hint_reading_a_key_only_a_runtime_write_creates_is_decided_at_launch`; all new tests mutation-checked by the subagents
        - docs: `docs/cli/sequence.md` (class table, `null`/`[]`/`""` paragraph), `docs/topics/execution-flow.md`, `docs/topics/flow-control/sequences.md`
        - checks: `just test-library providers::tests` 18/18, `preflight` 239/239, `hints` 8/8, `just test-cli sequence_task_providers::` 26/26, `review::tests` 14/14, `sequence_errors_cli::` 52/52, `empty_hint` 12/12 after the variant change, `just lint` pass
- work completed for 'R2-F4 empty provider hints bypass fallback' (plus R2-F3.7) at 07:38:03
- starting the work on 'R2-F2 explicit selection flags do not override circular hints' at 07:38:03 (delegated to a foreground subagent)
        - defect class: prediction rejected a lower-priority hint before applying the selection source that makes that hint irrelevant
        - `own_hints` now keeps a circular `agent`/`model` as data (`CircularHint`); `select_one` applies precedence per key: a circular `agent` fails only without a provider flag, and a circular `model` fails only when `model_decided_above_hints` finds no `--model`, provider-specific model variable (`CLAUDE_MODEL`/`ANTHROPIC_MODEL` for Claude) or `MODEL`
        - Failed, EmptyAgentList and WrongType hints still fail early under any override: a temporary probe showed composition evaluates the hint anyway, so letting them through only moves a known authoring error past earlier tasks
        - the selection diagnostic's hint now depends on the problem (`task_selection_hint` in `lib/src/composition/error/render/selection.rs`) and no longer recommends a flag the caller already passed
        - sibling sites: `mod.rs` gate (only gates `Unresolved`), `review_drafts` (raises no errors), `task_run.rs` `launch_target` and `target_launch.rs` `select_rebuilt_provider` (flag already wins first; a full run shows the composed prompt and the rebuild agree), library `ProviderPrediction::failure` (classifies only, doc updated): clean; group legs, nested and external tasks share `OwnHints::Task` and are tested; `--claude` and `--provider claude` both tested
        - discovered R2-F2.10: with a provider decided at launch, a circular `model` is excused only by `--model` or `MODEL`, not by a provider-specific variable, because the provider is unknown at preflight; clean by contract (spec §5 excuses a lower hint only for a *statically known* higher-precedence override, and otherwise requires the circular rejection), documented on `model_decided_above_hints`
        - tests (`sequence_task_providers::`): `a_provider_flag_replaces_a_circular_document_agent`, `a_provider_option_replaces_a_circular_document_agent`, `a_provider_flag_replaces_a_circular_with_agent`, `an_explicit_model_replaces_a_circular_document_model`, `an_explicit_model_replaces_a_circular_with_model`, `a_caller_agent_replaces_a_circular_document_agent`, `a_caller_model_replaces_a_circular_document_model`, `a_provider_flag_replaces_an_agent_decided_at_launch`, `a_circular_agent_with_no_override_fails_before_any_launch`, captured `MODEL` and `CLAUDE_MODEL` cells, `a_provider_flag_replaces_circular_agents_in_legs_and_nested_tasks`, `a_provider_flag_does_not_replace_a_circular_model`, `a_provider_flag_does_not_excuse_a_failed_or_empty_agent`; unit `review::tests::a_provider_flag_makes_a_circular_agent_irrelevant`, `a_higher_model_source_makes_a_circular_model_irrelevant`; mutation-checked (10 row tests and both hint tests fail with the old behavior)
        - docs: `docs/cli/sequence.md` (override table), `docs/topics/execution-flow.md`
        - checks: `just test-cli sequence_task_providers::` 40/40, `review::tests` 17/17, `wrap_provider_flags` 5/5, `just test-library providers::tests` 18/18, `just test-cli sequence_errors_cli::` 52/52, `just lint` pass
- work completed for 'R2-F2 explicit selection flags do not override circular hints' at 07:49:50
- starting the work on 'R2-F1 prompt composition retains an earlier launch identity' at 07:49:50 (delegated to a foreground subagent)
        - defect class: content that depends on the launch identity was rendered under a predicted or enclosing provider's environment before the effective launch identity was resolved, and the identity was never re-checked against a freshly read document, so a stale task-owned `AGENT`/`MODEL` overlay survived into the child
        - approach: a new no-shell, frontmatter-only read (`composition::read_selection_hints` in `lib/src/composition/prepare.rs`, built on Darkmatter's existing `only_frontmatter_surface`, the same mechanism the initialize-bootstrap read uses) runs before the body renders; `jit::compose_step_selecting` passes those hints to the existing `launch_target` and installs the chosen `AGENT`/`MODEL` in the composition environment (`replace_identity_env`); `compose_step` is unchanged for its other callers
        - `launch_target` now mirrors the launch rebuild's provider/model rule for every task (not only decided-at-launch ones), so a rewritten document launches with a matching child environment; a shared `identity_env` helper replaces two copies of the `AGENT`/`MODEL` insertion
        - trade-off accepted: re-rendering the body after selection was rejected because it would run body shells twice; an `agent`/`model` produced by a frontmatter `$(…)` still renders the body under the planned identity (documented)
        - sibling paths: prompt task direct, group leg, external task, inline and file-reference nested sequences: fixed (all reach `WrapperPromptRunner::run`); proxy (`loop_control.rs` rebuild) and body step (`jit.rs` `step_env_overrides`): clean controls; looping document and retry: unchanged, review observations, not reached by this fix
        - tests (`identity_rows!` in `cli/tests/l1/sequence_task_providers.rs`, each asserting spawned binary, `BODY_AGENT` in the delivered prompt and child `AGENT`): `identity_dynamic_with_agent`, `identity_dynamic_document_agent`, `identity_dynamic_with_model`, `identity_group_leg_agent`, `identity_rewritten_document_agent`, `identity_static_own_agent`, `identity_external_task_static_agent`, `identity_looping_prompt_static_agent`, `identity_proxy_target`, `identity_body_step`, `identity_inline_nested_dynamic_agent`, `identity_dynamic_with_agent_without_fallback`, `identity_external_task_dynamic_agent`, `identity_file_nested_dynamic_agent`; mutation check: without pre-render selection 9 defect rows fail (controls pass); with the old decided-at-launch-only `launch_target` gate rows 1.3 and 1.5 fail
        - doc drift corrected: `docs/cli/sequence.md` claimed `env.AGENT` names the fallback for a decided-at-launch task; `sequences.md` and `execution-flow.md` (new Mermaid flow) now describe selection before the body renders; `.claude/skills/claudine/SKILL.md` updated
        - R2-F4 consumer found by this run: `launch_trigger_schemas::a_sequence_step_document_with_an_invalid_control_fails_before_its_provider` used `agent: []` as its schema-only invalid control, which preflight now rejects first; the orchestrator switched it to `interactive: sometimes` (a control only the launch schema judges), keeping the test's intent; `[]` itself is covered by `empty_hint_frontmatter_list_fails_preflight_before_any_task_launches`
        - checks: `just test-cli sequence_task_providers::` 54/54, `with_file_provenance::` 15/15, `sequence_errors_cli::` 52/52, `review::tests` 17/17, `task_run::` 5/5, `jit::` 21/21, `target_launch` 38/38, `loop_control` 167/167, `launch_trigger_schemas` single test pass; `just test-library providers::tests` 18/18, `prepare::` 146/146; `just lint` pass
- work completed for 'R2-F1 prompt composition retains an earlier launch identity' at 08:06:57

### Completion Report

- **outcome: complete**: every finding is repaired and verified, and all required checks pass
- elapsed: 2026-10-18T07:03:02-07:00 → 2026-10-18T08:11-07:00, about 1 h 8 min
- parent findings: total 4 = fixed 4 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - R2-F4 counts as fixed: its defect (raw presence chose the category) is repaired at the source. Two children (R2-F4.2, R2-F4.4) are **disputed** on their expected result only. "Inherit Claude" for `agent: []` contradicts the shipped launch schema (`claudine.yaml` `string[](min(1))`, documented as invalid in `docs/topics/schemas.md`). Those cases now fail preflight with a located error before any task launches. The next review must cite authority that overrides the schema to reopen them.
- discovered rows: R2-F2.10 (clean by contract), R2-F3.7 (fixed), R2-F4.10 (fixed)
- nonblocking observations not repaired (review marks them out of scope): removed task models as planned fallbacks; retry composition environment (the retry path is untouched); a looping prompt document with an earlier-output provider hint (`run_looping_document` is untouched)
- accepted limitation, documented: an `agent`/`model` produced by a frontmatter `$(…)` still renders the body under the planned identity, because re-rendering would run body shells twice; the launch and child environment follow the command's answer
- open finding/child IDs: none
- verified repairs:
        - R2-F1: identity selected from a no-shell frontmatter read before the body renders; `launch_target` mirrors the launch rebuild for every task
        - R2-F2: provider and model precedence applied per key before a circular hint is rejected; the diagnostic hint depends on the problem
        - R2-F3: a failed `with` evaluation keeps its cause and becomes a located `Failed` prediction
        - R2-F4: category comes from the parsed hint; a static document/`with` `agent: []` gets a dedicated `EmptyAgentList` preflight failure
- changed files (all uncommitted):
        - library: `claudine/lib/src/composition/{hints.rs,types.rs,mod.rs,prepare.rs}`, `claudine/lib/src/composition/error/{mod.rs,render/selection.rs}`, `claudine/lib/src/composition/sequence/preflight/{mod.rs,providers.rs,providers/tests.rs}`
        - CLI: `claudine/cli/src/commands/wrap/sequence/{jit.rs,mod.rs,providers.rs,task_run.rs,review/tests.rs}`, `claudine/cli/tests/l1/{sequence_task_providers.rs,launch_trigger_schemas.rs}`
        - docs/skills: `claudine/docs/cli/sequence.md`, `claudine/docs/topics/execution-flow.md`, `claudine/docs/topics/flow-control/sequences.md`, `.claude/skills/claudine/SKILL.md`
        - `claudine/docs/topics/schemas.md` and `claudine/lib/tests/l1/sequence_schema.rs` were already modified by iteration 1 and were not touched here
- final verification (run by the orchestrator after every subagent finished, with none running):
        - `just lint` (claudine): exit 0 (all findings)
        - `just test` (claudine): 8889 passed, 9 skipped, exit 0, 51 s (all findings, including every new L1 test)
        - `just test-l2 sequence_review_screen`: 5 passed (R2-F4.9 review projection, acceptance criterion 7)
        - `just check-tier-coverage claudine`: 0 stranded
- cross-OS: no `just cross-check` run; the repair touches no paths, process spawning, or `#[cfg]` code, and the new path assertions match `to_portable_string` labels (forward slashes on every OS); CI's nightly covers Windows and WSL2
- no performance measurement was required; `deferred_perf_measurement` is not set

## Implementation of Review Findings #3

> **started at:** 2026-10-18T08:19:41-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- inventory of `review-3.md`:
        - one finding, R3-F1 (high), with fifteen child rows; `recurrence: true` (repeats the R2-F1 class)
        - `observations: []`, no blocked findings, no human-in-the-loop items
        - the review's `implemented` flag was already `false` before repairs started
        - the authority is spec section 5 ("Refresh selection at the existing preparation boundary so composition-time env.AGENT/env.MODEL and the child environment agree")
        - review constraint: do not render the whole document twice; resolve the frontmatter command once, select the identity, then render the body; each frontmatter and body command runs once

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R3-F1    | Shell-dependent selection still renders a stale launch identity | fixed | all required children fixed or clean; discovered retry sibling and dry-run regression fixed; full `just test` claudine 10447/10447 and darkmatter 10138/10138, both lints exit 0 |
| R3-F1.1  | Direct prompt document `agent: '$(echo goose)'`, enclosing Claude: body and child both identify Goose | fixed | `identity_shell_document_agent` pass; fails with in-compose selection disabled |
| R3-F1.2  | Direct prompt document `model: '$(echo fresh-model)'`: body also identifies fresh-model | fixed | `identity_shell_document_model` pass (child env, `--model` argv, body) |
| R3-F1.3  | Indirect document agent (choice field runs the command; agent reads it): both identify Goose | fixed | `identity_shell_indirect_document_agent` pass |
| R3-F1.4  | Concurrent group leg with the same agent command: both identify Goose | fixed | `identity_shell_group_leg_agent` pass |
| R3-F1.5  | External `kind: task` reference to the same prompt: both identify Goose | fixed | `identity_shell_external_task_agent` pass |
| R3-F1.6  | Inline nested sequence: both identify Goose | fixed | `identity_shell_inline_nested_agent` pass |
| R3-F1.7  | File-reference nested sequence: both identify Goose | fixed | `identity_shell_file_nested_agent` pass |
| R3-F1.8  | No enclosing fallback (no root agent): body identifies Goose | fixed | `identity_shell_document_agent_without_fallback` pass |
| R3-F1.9  | `with.model` holding a literal command-looking string: body matches the literal child model; no execution or reinterpretation | fixed | `identity_literal_command_looking_with_model` pass, value never executed; library `selecting_preparation_passes_a_command_looking_caller_model_as_data` |
| R3-F1.10 | `with.agent` holding the same literal string: located unknown-provider failure, no launch (clean control) | clean | `identity_literal_command_looking_with_agent_fails_without_a_launch`: located task failure, zero launches |
| R3-F1.11 | Document agent command plus `--claude`: body and child both Claude (clean control) | clean | `identity_shell_document_agent_under_a_provider_flag` pass |
| R3-F1.12 | Document model command plus `--model explicit-model`: both explicit-model (clean control) | clean | `identity_shell_document_model_under_an_explicit_model` pass |
| R3-F1.13 | Shared early reader: shell-bearing value must not bypass identity selection for either key; resolve effective identity before body render | fixed | selection now also runs inside the single full preparation via Darkmatter `compose_with_frontmatter_hook`, after frontmatter `$(…)` and before the body; the early read's skip no longer reaches the body (narrowing the skip by authored text would misread the indirect F1.3 case) |
| R3-F1.14 | Child environment/argv and completion consumers keep reporting the actual launch (clean control) | clean | every new row asserts child `AGENT`/`MODEL` and argv; root rows assert the "via Goose" completion line |
| R3-F1.15 | The fourteen R2-F1 non-shell identity cases still agree (clean control) | clean | all fourteen R2-F1 identity tests pass in `sequence_task_providers::` 68/68 |
| R3-F1.16 | Frontmatter command and body command each run exactly once | fixed | `identity_shell_selection_runs_each_command_once`: `runs.log` is exactly `fm`, `body` |
| R3-F1.17 | Discovered sibling: retry/resume reread (`materialize_harness_prompt` from `input_layers`) rendered attempt 2 with the stale identity | fixed | `identity_shell_document_agent_on_retry` pass; fails with the `input_layers` update removed |
| R3-F1.18 | Checked sibling: proxy from a shell-agent prompt to a static target | clean | temporary probe: goose then claude, each body agrees |
| R3-F1.19 | Checked sibling: `phase1c.rs`, `validate()`, `compose_with_late_collection` | clean | validation probes or body steps without a selector; they render no prompt for another provider |
| R3-F1.20 | Regression found by the orchestrator's full run: a predicted prompt task with no planned target and no `agent` (dry run, "no provider") failed at its turn with "it names no provider" | fixed | `launch_target` keeps `planned` for a task not decided at launch whose hints name no agent; `sequence_sources_cli::` 4 previously failing tests pass; full `just test` 10447/10447 |
| R3-F1.21 | Drift guard `composition_seams::compose_with_allowlist_holds_the_line`: new `compose_selecting` and `read_selection_hints` `compose_with` sites, stale `prepare_direct_on`/`prepare_inline_on` entries | fixed | allowlist updated with reasons (both are stages of the sanctioned composer); `composition_seams::` 19/19 |
| R3-F1.22 | Drift guard `dispatch_inventory::dispatch_inventory_matches_committed_file`: committed inventory stale (test-only `Provider::Claude` references in `review/tests.rs`) | fixed | regenerated with `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`; diff only adds exempt-candidate test references and counts |
| R3-D1    | Discovered, different class (not stale identity): a shell `agent` on a root body step's sequence document, a looping prompt, a proxy target, or a direct `claudine compose` fails closed with `agent resolution failed` and launches nothing | deferred | no stale prompt is delivered, so outside R3-F1's class; reported as pre-existing launch-time resolution behavior (not independently compared with the base branch); carried to the backlog for the follow-up pass |

### Progress

- starting the work on 'R3-F1: Shell-dependent selection still renders a stale launch identity' at 08:20:20
        - defect class: identity-dependent prompt rendering proceeds with a planned identity whenever the early selection read cannot decide a hint, and only the later launch corrects the child identity
        - delegated to one foreground rust-developer subagent, which owns the log entries for this finding
        - subagent started at 08:25 (rust-developer); skills loaded: claudine, rust, rust-testing, darkmatter
        - defect class (restated): identity-dependent prompt rendering proceeds with a planned identity whenever the early selection read cannot decide a hint, and only the later launch corrects the child identity
        - looked for an existing Darkmatter mechanism first: none fits; the compose pipeline (`darkmatter/lib/src/markdown/compose/pipeline/mod.rs::run_compose_pipeline_node`) resolves frontmatter (interpolation pass 1, schema, frontmatter `$(…)`, pass 2) and only then builds the effective state the body renders from, but exposes no point between the two; `only_frontmatter_surface` and shell pre-approvals do not carry command results into a second compose, and the run-local command cache dies with each compose
        - design chosen (option a): a call-scoped Darkmatter hook, `Markdown::compose_with_frontmatter_hook`, called once for the request root after its frontmatter is resolved and before its body renders, that may edit the environment the body renders with; the hook is a borrowed `&mut dyn FnMut` parameter, not a stored `ComposeOptions` field, so the CLI's non-`'static` identity selector can run inside the single compose and every frontmatter and body command still runs once
        - Darkmatter: `Markdown::compose_with_frontmatter_hook` + `ResolvedFrontmatterHook` (`darkmatter/lib/src/markdown/compose/mod.rs`); `pipeline/mod.rs::run_compose_pipeline_node` calls the hook for the root node only, after frontmatter interpolation pass 2 and before the effective state is built; a changed environment is also pushed into the node's `as_markdown()` scope (`nested.rs::align_environment`, the scope's options now behind a `Mutex`); transcluded children inherit it through the parent context; unit tests `frontmatter_hook_sets_the_environment_the_body_renders_with`, `frontmatter_hook_error_ends_the_compose`
        - Claudine library: `composition::ResolvedIdentitySelector` and `prepare_{direct,inline}_with_schema_selecting` thread a borrowed selector through the schema layer's `PrepareMode` (so its drop-and-retry rerun keeps it) into `prepare_{direct,inline}_on`; the selector parses the resolved frontmatter's hints, edits the composition environment, and a selector error ends the preparation before the body renders (returned as the selector's own `CompositionError`); the prepared `compose_context` adopts the selected environment
        - Claudine CLI: `jit::compose_step_selecting` keeps the early no-shell read (so frontmatter values reading `env.AGENT`/`env.MODEL` still see a decidable selection) and now also selects inside the full preparation; `IdentitySelector` is `Fn` (it only calls `launch_target`, which is pure); `replace_env_entries` is the one copy of the restore-or-unset rule
        - R3-F1.13 resolved by making the early reader's skip irrelevant to the body rather than narrowing it: narrowing by authored text would misread the indirect case (`agent: '{{ choice }}'` reads a still-unexpanded `$(…)` in a no-shell read) as an invalid provider; the early read still skips, but only frontmatter-time reads use its result
        - new L1 rows in `cli/tests/l1/sequence_task_providers.rs`: 27/27 identity tests pass; mutation (in-compose selection disabled) fails exactly the 10 defect rows (F1.1–F1.9, F1.16) and passes all 17 controls
        - library selector redesigned after the sweep found a retry sibling: `ResolvedIdentitySelector` now returns the override map and the library applies it (`composition::replace_env_overrides`, the one restore-or-unset rule, shared with the CLI's early read) and also writes it into the prepared `input_layers.env_overrides`, which every harness reread (retry, resume, loop refresh) composes from
        - sweep (each site probed with a temporary CLI test, then removed or kept):
                - `WrapperPromptRunner::run` (direct prompt, group leg, external `kind: task`, inline and file-reference nested): fixed, all go through `compose_step_selecting` (F1.1–F1.8)
                - retry of a shell-agent prompt task (`harness_orch/prompt.rs::materialize_harness_prompt` rereads from `input_layers`): **sibling found and fixed**: attempt 2 launched Goose with `BODY_AGENT=claude`; now `identity_shell_document_agent_on_retry` (fails with the `input_layers` update removed); resume shares the same reread
                - proxy from a shell-agent prompt to a static target: clean (`goose` then `claude`, each body agrees; probe removed)
                - root body step with a shell `agent` in the sequence document, looping prompt (`run_looping_document`) with a shell `agent`, proxy target whose own `agent` is a shell command, and direct `claudine compose` of such a document: all fail closed with `agent resolution failed` and launch nothing, so no stale prompt is delivered; this is a pre-existing limitation of the launch-time agent resolution outside sequence prompt tasks, not this defect class; recorded, not changed
                - `phase1c.rs` / `validate()` / `compose_with_late_collection`: validation probes or body steps without a selector; they render no prompt for a provider other than the planned one
        - docs: removed the accepted-limitation text from `claudine/docs/cli/sequence.md` (now an example of the shell case and the literal `with` case), `claudine/docs/topics/flow-control/sequences.md`, `claudine/docs/topics/execution-flow.md` (Mermaid flow now shows the second, in-compose choice), `.claude/skills/claudine/SKILL.md`; Darkmatter API documented in `darkmatter/docs/topics/compose-requests.md` (new section, and the "env does not change during a composition" sentence qualified) and `.claude/skills/darkmatter/compose.md`
        - remaining limitation (documented): a frontmatter value that itself reads `env.AGENT`/`env.MODEL` resolves before the frontmatter command runs, so it sees the identity the rest of the frontmatter names, not the command's answer
        - verification: `just test-cli sequence_task_providers::` 68/68; `just test-library preflight` 239/239; `just test-cli review::tests` 17/17; `just test-cli jit::` 21/21, `task_run::` 5/5, `with_file_provenance::` 15/15, `target_launch` 38/38; `just test-library prepare::` 146/146, `composition::schema::tests` 97/97 (includes the 3 new `selecting_preparation_*` and `replacing_env_overrides_*`); Darkmatter `cargo nextest run -p darkmatter --lib` over the touched compose modules 33/33, plus the earlier frontmatter-shell/nested filter 296/296
        - lint: `just lint` in darkmatter exit 0; `just lint` in claudine exit 0
        - process note: the mutation checks and sweep probes used `--no-fail-fast` on narrow filters (at most 27 tests) to see every row's verdict; no full-suite or unfiltered run was made
        - not run: full `just test` in either area, `just check-tier-coverage`, cross-OS; the new CLI tests sit in the existing Unix-gated `sequence_task_providers` module
- work completed for 'R3-F1: Shell-dependent selection still renders a stale launch identity' at 08:42:10
- orchestrator reconciliation started at 08:43:00
        - inspected the combined diff: Darkmatter `compose_with_frontmatter_hook` is called once for the root node, after frontmatter resolution and before the effective state the body renders from; the Claudine selector threads through `prepare_{direct,inline}_with_schema_selecting` and updates `input_layers.env_overrides` so rereads (retry/resume) keep the selection
        - first full `just test` (claudine): `composition_seams::compose_with_allowlist_holds_the_line` failed on new `compose_with` sites; `read_selection_hints`'s site predates this cycle, so the guard's baseline was already stale; allowlist updated (R3-F1.21)
        - second full run: `dispatch_inventory::dispatch_inventory_matches_committed_file` failed on a stale committed inventory (test-only references from earlier iterations); regenerated (R3-F1.22)
        - third full run: four `sequence_sources_cli::` dry-run tests failed: a predicted task with no planned target and no `agent` hit `classify_agent_resolution` in the rewritten `launch_target`; fixed by keeping `planned` for a non-at-launch task whose hints name no agent (R3-F1.20); `just test-cli 'sequence_sources_cli:: sequence_task_providers::'` 91/91
        - fourth full run green; see the completion report
        - cross-OS check not run: the change adds an in-memory environment hook and selection wiring with no path, process, or OS-specific code; the new CLI tests sit in the existing Unix-gated `sequence_task_providers` module (recording-shell stubs)
        - R3-D1 recorded as deferred: a different class (fails closed, no stale prompt), reported by the subagent as pre-existing; not compared with the base branch in this cycle

### Completion Report

- **outcome: complete**: elapsed 08:19:41 → 08:59 (about 39 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - R3-F1 fixed: children F1.1–F1.9, F1.13, F1.16, F1.17, F1.20–F1.22 fixed; F1.10–F1.12, F1.14, F1.15, F1.18, F1.19 clean
        - R3-D1 (discovered, outside R3-F1's defect class, not a review finding) is deferred to the backlog: a shell `agent` on a root body step's sequence document, a looping prompt, a proxy target, or a direct `claudine compose` fails closed with `agent resolution failed` and launches nothing; next action: decide whether launch-time resolution outside sequence prompt tasks should run the frontmatter command before selecting
- verified repair: a prompt whose `agent`/`model` comes from a frontmatter `$( … )` now composes its body with the identity that launches, because selection runs inside the single compose between the resolved frontmatter and the body; no document renders twice, and the frontmatter and body commands each run once; resolved task-input data that looks like a command stays literal data
- documented limitation (in `claudine/docs/cli/sequence.md` and the Darkmatter compose-requests page): a frontmatter value that itself reads `env.AGENT`/`env.MODEL` resolves before the frontmatter command runs, so it sees the identity the rest of the frontmatter names
- changed files this cycle:
        - Darkmatter: `darkmatter/lib/src/markdown/compose/{mod.rs,pipeline/mod.rs,nested.rs,context/options.rs,tests/shell.rs}`, `darkmatter/docs/topics/compose-requests.md`, `.claude/skills/darkmatter/compose.md`
        - Claudine library: `claudine/lib/src/composition/{prepare.rs,mod.rs,schema/mod.rs,schema/translate.rs,schema/tests.rs}`
        - Claudine CLI: `claudine/cli/src/commands/wrap/sequence/{jit.rs,task_run.rs}`, `claudine/cli/tests/l1/{sequence_task_providers.rs,composition_seams.rs}`, `claudine/docs/providers/dispatch-inventory.json`
        - docs: `claudine/docs/cli/sequence.md`, `claudine/docs/topics/flow-control/sequences.md`, `claudine/docs/topics/execution-flow.md`, `.claude/skills/claudine/SKILL.md`
- final verification (macOS):
        - `just test` in `claudine/`: 10447 passed, 9 skipped, exit 0 (all R3-F1 rows, R3-F1.20–F1.22)
        - `just lint` in `claudine/`: exit 0
        - `just check-tier-coverage claudine`: 0 stranded
        - `just test` in `darkmatter/`: 10138 passed, 10 skipped, exit 0 (the hook and its two unit tests)
        - `just lint` in `darkmatter/`: exit 0
        - `just test-l2 sequence_review_screen` in `claudine/`: 5/5 (review-screen consumer unchanged)
        - subagent-scoped runs: `sequence_task_providers::` 68/68 (F1.1–F1.17), `preflight` 239/239, `review::tests` 17/17, `composition::schema::tests` 97/97
- process note: the subagent used `--no-fail-fast` on narrow filters (at most 27 tests) for mutation checks; no full-suite run used it
- remaining finding IDs: none open under R3-F1; R3-D1 deferred (outside the finding)

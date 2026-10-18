---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-26-task-inputs-and-providers/spec.md
plan: claudine/features/2026-10-26-task-inputs-and-providers/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 was inventory and a spike only; no source, doc, or skill file changed. Read the
    `## Phase 1` section of this log before starting: it holds file:line maps for every task
    `params` reader/writer, every `with` evaluator, and every planned-target consumer, plus the
    migration inventory for Phase 5.

    Rulings settled (no author input needed):
    - Rule 1: every dependency spec is implemented; Phase 4 is unblocked.
    - Rule 2: the error family is already `LifecycleWith*` (NotMapping, DynamicKey, WholeMapping,
      EvaluationFailed, Unsupported). Rename nothing. `LifecycleWithUnsupported`'s doc and message
      still say "only `proxy` and `retry`" and omit `prep`; fix that while in the area.
    - Rule 14: `claudine/schemas/` is the only schema definition. The embedded bundle is the
      explicit `include_str!` list in `claudine/lib/src/composition/schema/bundle.rs`
      (`CLAUDINE_SCHEMA_FILES`, `CLAUDINE_TRIGGER_FILES`). `claudine/docs/schemas/*.yaml` are empty
      design drafts no reader loads, and `darkmatter/docs/schemas/` has no claudine.yaml: nothing
      to mirror.

    Facts that differ from the plan's assumptions:
    - Task `params` already resolve relative file values from the authoring file
      (`cli/src/commands/wrap/sequence/task_run.rs:772-795` builds origin-anchored
      `CallerInputRecord`s). The task-side provenance work is mostly the key rename.
    - `proxy.with`/`retry.with` overlays carry no origin and resolve against the TARGET document
      (layered as `OverrideOrigin::Data`). `prep.with` resolves from the LAUNCH directory
      (`cli/src/commands/prep_target.rs` hands it over as `shared.set`). Both must move to
      authoring-file-relative; the natural carrier is a `CallerInputRecord` built from the
      authoring document's `FileResolutionContext`, layered below caller setters.
    - Why a task's own `agent:` is ignored today: `task_run.rs:611-630` `document_target` passes
      the step's planned target into `composition/target.rs:99-106` `resolve_execution_target`,
      which reuses it verbatim.
    - `cli/tests/l1/wrap_sequence_composition.rs::sequence_live_shell_only_no_agent_still_aborts_headless`
      (around line 538) asserts the abort that AC6 removes; Phase 3 must invert it.
    - `cli/tests/l1/sequence_errors_cli.rs:133-144` asserts `params` is rejected on a shell task;
      after the rename it would pass silently unless switched to `with:`.
    - `claudine/schemas/lifecycle.yaml` already has `cancelled-stack`, `loop`, and two typed
      container levels (`task`/`group`, `-2`); its `task`/`task-2` still declare `params`.
    - The 2026-09-27 amendments file is already gone; Phase 5 has nothing to delete.

    Phase 4 spike results (full construction in the log):
    - Spec section 6's trigger `match:` (a property-level union) is REJECTED by the loader. Use
      `match: { any: [ {sequence: string(required)}, {sequence: number(required)},
      {sequence: boolean(required)}, {sequence: object(required)},
      {sequence: "unknown[](required)"} ] }`.
    - Root `fail_fast` is NOT reported: the trigger payload leaves the root open. AC9 needs it
      reported by name, so find a construction (for example, declare `fail_fast` with a type that
      always fails and a "removed" description, if the engine supports one) and record the choice.
      Group `execution` and `max_paralel` ARE reported (groups are closed).
    - Number-to-string coercion applies at level 1 only (`name: 7` passes at L1, fails at L2/L3;
      `sequence: 42` may coerce through the `string` arm). Assert the real outcome of AC9's
      `sequence: 42` case and use lists/objects for invalid values that must fail at every level.
    - The sequence trigger adds about 0.8 s (release) / 3.5 s (debug) per validated document. Keep
      schema tests in-process; one debug `md` spawn per fixture costs about 8 s, which breaks the
      5 s L1 budget. Also measure the launch-verdict cost of `claudine sequence --dry-run` on a
      shipped sequence before and after, and log it.
---

# Implementation Log for 2026-10-26-task-inputs-and-providers (5 phases)

## Phase 1

### Dependency check and rulings (Rules 1, 2, 14)

All dependency and related specs are implemented (`implemented: true`):

| Spec | Location | State |
| --- | --- | --- |
| `2026-10-20-lifecycle-grammar` | `_completed` | implemented |
| `2026-10-21-claudine-schemas` | `_completed` | implemented |
| `2026-10-23-shell-template-approval` | `_completed` | implemented |
| `2026-10-24-sequence-containers` | `_completed` | implemented |
| `2026-10-25-loops-and-flow-control` | active (review-1 present, uncommitted spec edit in working tree — not touched) | implemented |
| `2026-10-14-union-type-constraints` | `_completed` | implemented |
| `2026-10-13-named-type-presence-and-local-names` | `_completed` | implemented |
| `2026-10-16-core-schema-types` | `_completed` | implemented (no in-flight edits to coordinate with any more) |

- **Rule 1 (dependency gate) — resolved, no ASK needed.** The schema and loop
  specs have landed, so Phase 4 is unblocked. `claudine/schemas/lifecycle.yaml`
  already exports `stack`, `initialize-stack`, `loop` (line 124),
  `cancelled-stack` (line 299, typed at all five conditional levels), and the
  container types `step`/`task`/`group` plus `step-2`/`task-2`/`group-2`
  (two typed container levels). Phase 4's "add `cancelled-stack`" task is
  therefore already done by the dependency; Phase 4 reuses these and extends
  the container expansion to three typed levels in `sequence-types.yaml`.
  Note: `lifecycle.yaml`'s `task` type still declares
  `params: "object -> parameters passed to the executable"` (and its `task-2`
  copy); the rename must update it.
- **Rule 2 (error-family names) — resolved.** `2026-10-20-lifecycle-grammar`
  already renamed the family. The live variants are `LifecycleWithNotMapping`,
  `LifecycleWithDynamicKey`, `LifecycleWithWholeMapping`,
  `LifecycleWithEvaluationFailed`, and `LifecycleWithUnsupported`; no
  `LifecycleProxyWith*` variant remains (only the unrelated
  `LifecycleProxyWithoutOwningCoordinator`). Phase 2 keeps this family and
  renames nothing.
- **Rule 14 (schema file ownership) — resolved.** `claudine/schemas/` is the
  only definition. `claudine/lib/src/composition/schema/bundle.rs` embeds the
  files with an explicit `include_str!` list (`CLAUDINE_SCHEMA_FILES`) and
  names the triggers in `CLAUDINE_TRIGGER_FILES`; both lists must gain
  `sequence.yaml`/`sequence-types.yaml` in Phase 4 (code, not a build or
  `just` step). `claudine/docs/schemas/{claudine,compose,inline-compose}.yaml`
  are **empty files** and `action.yaml` there is an old draft;
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs` classifies
  `claudine/docs/schemas` as "design drafts, not loaded by any reader". They
  are not mirrors and need no sync. `darkmatter/docs/schemas/` contains no
  `claudine.yaml` (the plan's premise was wrong); nothing to coordinate there.
  `claudine/cli/tests/l1/shipped_prompts.rs:15` and
  `darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs:100` read
  `claudine/docs/schemas` (the partials) and are unaffected.
- `claudine/schemas/README.md` says "the five files" and must be updated in
  Phase 4 to seven.

### Test baseline (2026-10-17, macOS host)

Run before any change, sequentially, with no subagent running tests:

| Gate | Result |
| --- | --- |
| `claudine/`: `just test` | 10321 passed, 9 skipped, 13 slow (196.7 s wall); exit 0 |
| `claudine/`: `just lint` | exit 0 |
| `darkmatter/`: `just test schema` (lib, CLI, DMLS tests whose names contain `schema`) | 2262 passed; exit 0 |

No pre-existing failures. Unrelated working-tree changes, left untouched:
`claudine/features/2026-10-25-loops-and-flow-control/spec.md` (modified) and
`claudine/features/2026-10-25-loops-and-flow-control/review-1.md` (untracked).
The host load average was 57–66 during the run (other sessions); the suite
was green regardless.

### Contract inventory

Paths are relative to `claudine/`. Line numbers are as of this phase's start.

#### Task `params` readers and writers

- **Reserved-key lists** (`lib/src/composition/sequence/reserved.rs`):
  `TASK_OPTION_KEYS` (`:25-28`, `"params"` at `:26`) and
  `allowed_task_options()` (`:70-79`; `Prompt` arm `:72` is
  `["name","params","operation","flow","when"]`). Module doc `:4-6`.
- **Option rejection.** `normalize.rs:228-262` raises
  `CompositionError::SequenceInvalidTaskField { index, field, executable }`
  (`error/mod.rs:1694-1706`, "sequence step at index {index} sets `{field}`,
  which is not valid for a `{executable}` task") for an option outside
  `allowed_task_options`. Task/group/file references are checked in
  `preflight/mod.rs:2186-2240` (`parse_task_object`) with
  `SequenceReferenceInvalid { context: "task", .. }`. `grammar.rs:580`
  `removed_task_key` is the existing hard-rename pattern if a `params` hint is
  ever wanted (not planned: `params` becomes ordinary step data).
- **Fields.** `preflight/shape.rs:38-40` `PreflightTask.params` (unevaluated;
  origin `PreflightTask.origin_path`); `preflight/mod.rs:160-164`
  `PromptReference.params` + `deferred_params`; `task/mod.rs:205-212`
  `PromptTaskRequest.{set_overrides, params, params_origin_path}`;
  `task/mod.rs:434` `TaskExecution.user_setters`.
- **Flow.** `preflight/mod.rs:797` reads `option_map(options, "params")` →
  `record_prompt_reference` (`:1910-1958`, evaluates with
  `reapproval::compose_approval_value` against the authoring file's
  `source_context_for(origin)`, defers late values) → `:877` stores it.
  `task/mod.rs:910-932` `prompt_request` → `evaluate_params()` (`:1410-1426`,
  raises `SequenceTaskParamReserved`) → `layered_overrides` (`:1430-1439`);
  `:820-840` re-resolves at the task's turn. `task/sequence.rs:177,477` clear
  params for container body/proxy tasks. `lifecycle/prewarm/sequence.rs:107`
  treats param keys as late. CLI: `wrap/sequence/mod.rs:142-149` applies
  `reference.params` as data during discovery compose; `task_run.rs:336,360`
  scans them for ctx reads; **`task_run.rs:772-795` already turns them into
  origin-anchored caller records** (`CallerInputLayers::from_caller_overrides`
  with `derive_source(&request.params_origin_path)`), so task inputs already
  follow the authoring-file rule; only the key changes.
- **Precedence today** (`task/mod.rs:1434-1439`), lowest first: document
  frontmatter → task `params` (`OverrideOrigin::Data`) → sequence user setters
  (`Authored`) → runtime mutation snapshot → reserved overlay. Matches Rule 4.

#### `with` evaluators (proxy / retry / prep)

- **Parse** (`lifecycle/parse.rs`): `:893-903` accepts `with` on
  `proxy|retry|prep` only, else `LifecycleWithUnsupported` (`error/mod.rs:
  1026-1037`; **its doc and message are stale** — they say "only valid on
  `proxy` and `retry`" and omit `prep`; Phase 2 fixes that). `:1129-1195`
  `parse_with` raises `LifecycleWithWholeMapping`, `LifecycleWithNotMapping`,
  `LifecycleWithDynamicKey`. Types in `lifecycle/actions.rs`: `WithOverlay`
  (`:528`), `WithOverlayError` (`:359`), `type_with_value` (`:588`). This is
  the evaluator Phase 2 should share with task `with`.
- **Evaluate** (`lifecycle/executor.rs`): proxy `:2067-2078`, retry
  `:2083-2104`, `run_prep` `:2150-2185`, all through `resolve_with_overlay`
  (`:2194-2220`) and the atomic `walk_with_overlay` (`:2223-2251`,
  `LifecycleWithEvaluationFailed`). Result is a plain `IndexMap<String,
  Value>` with **no origin**.
- **File resolution today:**
  - `proxy.with` / `retry.with`: stored in `harness_orch/types.rs:54,63`,
    merged by `wrap/overlay.rs:17`, handed over as
    `PrepareOptions.proxy_overlay` and layered by
    `LayeredOverrides::proxy_overlay` (`runtime_state.rs:321`) as `Data` with
    **no `CallerInputRecord`**, so a relative file value resolves against the
    **target** document.
  - `prep.with`: `PrepCall::overlay_as_data()` (`prep.rs:46`) →
    `cli/src/commands/prep_target.rs:39-50` sets it as `shared.set`, so the
    entry point captures it as a caller setter with the **launch** context
    (`compose/prep.rs:234-237`, `sequence.rs:392-395`). **`prep.with` today
    resolves from the launch directory**, which the plan did not mention; Phase
    2 moves it to authoring-file-relative too.
- **Caller provenance types to reuse:** `composition/types.rs:584`
  `CallerInputLayers` / `:642` `from_caller_overrides(set, origin:
  FileResolutionContext)` → Darkmatter `CallerInputRecord` /
  `CallerInputRecords` (`darkmatter/lib/src/markdown/compose/context/
  options.rs:27,53`), applied in `prepare.rs:103,444` and
  `schema/translate.rs:217`. The origin carrier for `with` values should be a
  `CallerInputRecord` built with the authoring document's
  `FileResolutionContext`, layered below caller setters.

#### Planned-target / provider consumers (all under `cli/src/commands/wrap/`)

- **Gate:** `sequence/mod.rs::execute_sequence_inner` (`:379`); `:482-484`
  document-level hints only; `:629-649` `classify_agent_resolution` →
  `AgentResolutionFailed` (`lib/.../error/mod.rs:283`) when headless. Runs
  before the graph is inspected, so shell-only sequences abort today.
- **Drafts:** `mod.rs:651-732`, one `SequenceStepDraft` per step from the
  document's hints.
- **Review screen:** `sequence/review.rs` (`is_review_eligible :23`,
  `baseline_targets :35`, `merge_reviewed_targets :66`, `review_targets :93`,
  `needs_review :116`, `live_targets :129`), called from `mod.rs:754-781`;
  UI `selection_ui::review_sequence`.
- **Status/report:** `iterate.rs:729-760` `emit_step_status(.., provider)`
  from `StepOutcome` ← `WrapperPromptRunner::take_provider()` (`task_run.rs:
  191,519,604`, last-writer-wins per step). `report.rs:21` names no provider.
- **Env:** `jit.rs:131` `step_env_overrides` (sets `AGENT`, `MODEL`, `YOLO`),
  called from `iterate.rs:375-386` and `phase1c.rs:229`;
  `sequence/resolve.rs:142` `StepSelection::for_document`;
  **`task_run.rs:611-630` `document_target` hands the step's planned target to
  `composition/target.rs:99-106` `resolve_execution_target`, which reuses it
  verbatim — this is why a task document's own `agent:` is ignored today.**
- **Launch rebuild:** `harness_orch/loop_control/target_launch.rs:328`
  `rebuild_launch_identity` → `:524` `select_rebuilt_provider`;
  `loop_control.rs:1397` `apply_target_env_overrides` (`:1200`, `:1689`).

#### Acceptance criteria → tests

| AC | Existing coverage | To add |
| --- | --- | --- |
| 1 `with` on tasks | `sequence/tests.rs::shell_step_rejects_prompt_only_task_option`, `preflight/tests.rs::meaningless_task_field_is_rejected`, `cli/tests/l1/sequence_errors_cli.rs::a_task_field_meaningless_for_the_executable_is_rejected`, `task/tests.rs::params_are_evaluated_just_in_time_against_the_effective_state` | rename to `with`; `params` read as `seq.params`; rejection on `side_effect`/`group`/`task`/nested `sequence`; `task:` + sibling `with` |
| 2 provenance | `cli/tests/l1/compose_caller_file_provenance.rs` (task params vs CLI origins, proxy sibling paths, caller outranks `proxy.with`, malformed/eager/lazy failures, second hop), `executor/tests/{proxy,retry}_with_evaluation.rs` atomicity | authoring-file anchoring for `proxy`/`retry`/`prep` `with` with target in another directory; `kind: task` file elsewhere; lazy/remote/ambiguous arms; forwarded anchored value; chained schema change; prep atomicity |
| 3 shape errors | `lifecycle/tests/action_shape_control.rs` (proxy rows), `tests/grammar.rs`, `tests/break_prep.rs::prep_with_follows_proxy_with_rules`, `error/tests.rs` | every row for task `with`, `retry.with`, `prep.with`, source-located |
| 4 tasks name providers | none (nearest `sequence_groups.rs`, `sequence_planned_model.rs`) | looping sequence, no doc `agent`, stub per task |
| 5 precedence | `task/tests.rs::layered_overrides_run_params_then_setters_then_mutations_then_overlay`, provenance shadow tests, `sequence_planned_model.rs`, `target_launch/tests.rs` provider-switch tests | `agent=gemini` vs `with.agent`; nested sequence's own `agent`; list hints; model env order |
| 6 early failure | `wrap_sequence_composition.rs::sequence_live_*_aborts_*` (document-level) | per-task uninstalled provider, false `when`; **invert `sequence_live_shell_only_no_agent_still_aborts_headless` (`:538`), which asserts the behavior the spec removes** |
| 7 review (L2) | `cli/tests/level2/level2_sequence_review_screen.rs` (4), L3 keys test, `sequence/review/tests.rs` | read-only own-provider rows; edited row launches |
| 8 decided at launch | none (related: `target_launch/tests.rs::a_refreshed_unavailable_scalar_agent_refuses_like_direct_selection`) | `seq.outputs`, runtime-mutated, concurrent-leg snapshot, purity stub, per-attempt identity, skipped/blocked claim none, pre-spawn failure |
| 9 schema | `lib/tests/l1/claudine_schemas.rs::{ac5_trigger_selection, sequence_container_schema_contract}`, `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`, `darkmatter/cli/tests/l1/schema_validate_shipped_artifacts.rs` | `sequence.yaml` trigger, example fixtures, `max_paralel`/root `fail_fast`, malformed `sequence:`, malformed task `with`, three-level + deeper fixtures, installed bundle |
| 10 sweep | n/a | see migration inventory |

### Migration inventory

No live file under `prompts/`, `claudine/prompts/`, or `darkmatter/prompts/`
uses task `params:` (`prompts/review-loop.md` was deleted in a6134aac1). No
existing test or shipped prompt depends on target-relative `proxy.with` file
values, so the provenance change rewrites no shipped `with:` map; it is a docs
change plus new tests.

**Convert** (task-input `params:` → `with:`):

| Path | Hits | Note |
| --- | --- | --- |
| `claudine/cli/tests/fixtures/frozen_prompts/prompts/review-loop.md` | 18 | only remaining copy of review-loop; read by `cli/src/commands/wrap/sequence/review/tests.rs:23`; `agent` inside `params` feeds prediction, so convert despite "frozen" header |
| `claudine/cli/tests/l1/sequence_groups.rs` | 4 (`:280,284,331,488`) | |
| `claudine/cli/tests/l1/agent_text_is_data.rs` | 3 (`:165,206,834`) | |
| `claudine/cli/tests/l1/sequence_shell_templates.rs` | 3 (`:265,447,717`) | test named `sequence-lifecycle-params` |
| `claudine/cli/tests/l1/compose_caller_file_provenance.rs` | 3 (`:493,543,590`) | relative path, already authoring-relative; seed for AC2 |
| `claudine/cli/tests/level2/level2_file_resolution_capture.rs` | 1 (`:586`) | |
| `claudine/cli/tests/l1/inline_completion_lifecycle.rs` | `:871-931` | fixture + comment |
| `claudine/cli/tests/l1/sequence_errors_cli.rs` | `:133-144` | must switch to `with:` and the new error, else it passes silently once `params` is data |
| `claudine/lib/tests/l1/claudine_schemas.rs` | `:1199` | |
| `claudine/lib/src/composition/sequence/tests.rs` | 5 (`:1110-1144`) | includes shell rejection |
| `claudine/lib/src/composition/sequence/task/tests.rs` | ~20 (`:3384-4364`) | error-path strings `params.which` |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | ~8 (`:873,1526-1547,2043-2083`) | |
| `claudine/lib/src/composition/lifecycle/prewarm/sequence/tests.rs` | `:178` | |
| `claudine/cli/src/commands/wrap/sequence/task_run/tests.rs` | `:86` | struct field |
| `claudine/schemas/lifecycle.yaml` | `:406,454` | `task`/`task-2` types |
| `claudine/docs/cli/sequence.md` | `:74,121,147,161` | |
| `claudine/docs/topics/flow-control/sequences.md` | `:156,160,172,331,620` | skill `topics` symlinks here — edit once |
| `claudine/docs/topics/composition.md` | `:1603` | |
| `claudine/docs/topics/flow-control/lifecycle.md` | `:618` | |
| `claudine/docs/topics/pre-flight-checks.md` | `:369` | |
| `.claude/skills/claudine/SKILL.md` | `:40` | |
| `.claude/skills/claudine/architecture.md` | `:666` | |
| `.claude/skills/claudine/cli-commands.md` | `:87` | |

**Rewrite-relative** (docs stating or implying the old `with` file rule):
`claudine/docs/topics/composition.md:1052-1059`;
`claudine/docs/topics/flow-control/flow-control-reference.md:89-175`
("Passing values with `with:`"); `claudine/docs/topics/flow-control/
flow-control.md:185,244-260,354-373` (one sentence each);
`.claude/skills/claudine/architecture.md:866-873` (states a launch-relative
rule for `proxy.with`, contradicting both old docs and the spec).

**Check, no change expected:** `prompts/implement.md:34-37,50-51` builds `log`
from `dirname(spec)`; `spec` is an eager file and should already be absolute.

**Preserve:** `.claude/skills/claudine/timeline.md:36` (evidence; add a new
entry instead), `claudine/docs/rollout-strategy.md:53` (describes the rename),
all HookAction `Bash { params }`, JSON-RPC/LSP/MCP `params`, Rust function
parameters, OpenCode `chat.params`, Darkmatter `::directive [params]`;
`proxy`/`retry`/`prep` `with:` tests whose router and target share a
directory (`compose_caller_file_provenance.rs:203,242,277`), expression
arguments (`agent_text_is_data.rs:592,627,678`), shell strings
(`level2_lifecycle_control.rs:6951`), and scalar/empty maps elsewhere.

**Amendments file.** `claudine/features/2026-09-27-sequence-improvements/
amendments-provider-and-with.md` is already gone from the worktree (removed in
adfae0873 / ea7c217ff). Phase 5's "remove the amendments file" task has
nothing to delete. The spec's sentence saying "that file is removed in this
change" is now out of date; per repository rules the spec is left as written
and this departure is recorded here.

### Bounded-expansion schema spike (Wave 2)

Run once on this macOS host in a throwaway git repository under `$TMPDIR`
(triggers are discovered only for a document inside a repository, so the
spike placed the six `claudine/schemas/*.yaml` files plus the two spike files
in that repository's root `schemas/` folder). No repository file was edited.
Validated with `target/release/md` and `target/debug/md` through
`md schema validate --format json` and `md schema triggers`.

**Working construction.** `sequence-types.yaml` (`kind: schema`) defines
`steps`/`step`/`task`/`group`, then `-2` copies with references bumped
(`task.sequence: steps-2`, `group.tasks: "task-2[](required; min(1))"`, …),
then `-3` copies whose containers are the untyped boundary
(`task-3.sequence: ["unknown[] -> untyped boundary …", "string -> a sequence
file reference"]`, `group-3.tasks: "unknown[](required; min(1))"`). Lifecycle
types are imported (`loop@./lifecycle.yaml`, `stack@…`, `initialize-stack@…`,
`cancelled-stack@…`); `steps` is `["step[] -> …", "string -> …"]`; `step` is
`["string -> …", task]`; `task` keeps `"<string>": "any -> authored step
data"`. Level-1 task and group as loaded:

```yaml
task:
    name: "string -> the step or leg name"
    when: "expression -> run this task only when true"
    with: "object -> frontmatter values for the document this task runs"
    prompt: "file -> a prompt document to compose and run"
    shell: [shell, "shell[]"]
    side_effect: "object -> one lifecycle action"
    group: [group, "string -> a kind: group file, or name@catalog-file"]
    task: "file -> an externalized kind: task file"
    sequence: steps-2
    timeout: "string -> per-command shell timeout"
    operation: string
    flow: string
    loop: "loop@./lifecycle.yaml -> only beside an inline sequence list"
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml        # also blocked/success/failure/finalize
    cancelled: cancelled-stack@./lifecycle.yaml
    "<string>": "any -> authored step data"
group:
    name: string(required)
    description: string
    variables: "object -> read as group.<variable>"
    tasks: "task-2[](required; min(1))"
    max_parallel: number(integer; min(1))
    fail_fast: "boolean -> true (default) = the legs are atomic"
    operation: string
    flow: string
    loop: loop@./lifecycle.yaml
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml        # also blocked/success/failure/finalize
    cancelled: cancelled-stack@./lifecycle.yaml
```

The trigger that loads:

```yaml
kind: trigger-schema
match:
    any:
        - sequence: string(required)
        - sequence: number(required)
        - sequence: boolean(required)
        - sequence: object(required)
        - sequence: "unknown[](required)"
$schema:
    sequence: steps@./sequence-types.yaml
    template: "object -> values merged into every step (formal sequence documents)"
    loop: loop@./lifecycle.yaml
    initialize: initialize-stack@./lifecycle.yaml
    start: stack@./lifecycle.yaml
    blocked: stack@./lifecycle.yaml
    success: stack@./lifecycle.yaml
    failure: stack@./lifecycle.yaml
    cancelled: cancelled-stack@./lifecycle.yaml
    finalize: stack@./lifecycle.yaml
```

**Confirmations:**

1. No cycle rejection; `md schema triggers` reports `sequence.yaml — matched`.
2. Bare local names resolve (errors reported at typed paths three levels
   down, e.g. `/sequence/0/sequence/0/sequence/0/name`).
3. `step[]` list-of-union accepted (bare string step + task + group → valid).
4. Untyped boundary: a seven-level task/group document with unknown keys at
   level 7 is valid; a level-4 task with `name: [x]`, `execution:`, `with: [1]`
   is valid.
5. Rejections at each typed level: group `execution:` at L1, L2, L3 (anyOf
   error at `/sequence/0/group`, …); task `name: [x]`/`name: 7` at L2/L3;
   L3 `with: [1]`; group `max_parallel: 0`; `cancelled: [{retry: 2}]` at L1
   and L3; `loop: {max: 0}`. With both Claudine triggers matched, the group
   `execution:` error is still reported.
6. DMLS uses the same library roots and compile/validate path
   (`darkmatter/dmls/src/overlay/mod.rs:630` `trigger_registry` →
   `overlay/schema.rs:958` `with_trigger_registry`/`effective_for`; `md schema
   validate` → `triggers::scan_with_embedded` → the same `effective_for`).
   DMLS discovers triggers only inside a workspace folder. DMLS itself was not
   driven over stdio in the spike; Phase 4's DMLS L1 test covers it.

**Timing** (one document, wall time, three samples):

| Document | release `md` | debug `md` |
| --- | --- | --- |
| no trigger | 1.44 s | 4.86 s |
| `claudine.yaml` trigger only | 1.44 s | 5.16 s |
| `sequence.yaml` trigger, valid three-level doc | 2.19–2.22 s | 8.3–8.4 s |

The sequence trigger adds about 0.8 s (release) / 3.5 s (debug) per document;
13 documents in one `md` call took 56 s in debug (cost is per document, not
amortized).

**Findings that change Phase 4 (departures from the spec, recorded here; spec
left as written):**

- **Spec §6's `match:` does not parse.** A property-level union in a match
  condition is rejected (`property sequence in a match condition may not be a
  property-level union; use nested any: instead`), which would make the
  trigger unparseable for every document. Use the `any:` form above (same
  form as `claudine.yaml`).
- **Root `fail_fast` is not reported by this construction.** The trigger
  payload leaves the document root open, so an undeclared root key is
  accepted; group `execution`/`max_paralel` are reported because group objects
  are closed. AC9 requires root `fail_fast` to be reported by name, so Phase 4
  needs an explicit construction for it (see `message_to_agent`).
- **Number→string coercion is depth-dependent.** `name: 7` and `when: 5` are
  accepted at L1 (and `sequence: 42` coerces through the `string` arm) but
  rejected at L2/L3, following Darkmatter's documented nested-coercion limit.
  Fixtures should use non-coercible invalid values (lists, objects) when one
  test must fail at every level, and `sequence: 42` "fails payload validation"
  only if the `string` arm is not taken; Phase 4 must check what AC9's
  `sequence: 42` case actually produces and assert the real outcome.
- **Cost.** Validation-heavy L1 tests must stay in-process (or batch several
  documents per spawn) to keep under the 5 s L1 budget; one debug `md` spawn
  per fixture costs about 8 s.

### Phase 1 close

- **Behavior changed this phase:** none. Phase 1 changed no source, doc,
  schema, or skill file (only this log and the plan's checkboxes and
  frontmatter), so it added no tests. The requirement-to-test mapping for the
  later phases is the "Acceptance criteria → tests" table above. Input
  robustness matrix: not exercised this phase (no reader changed).
- **Gate decision:** `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: ["darkmatter"]`, `change_class: package`. The cause is
  `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`, which scans
  `claudine/features/` (each changed Markdown file under `claudine/features/`
  selects it, including the unrelated loops-spec edits). Because `packages`
  is non-empty, the gates ran:
  - `claudine/`: `just test` → 10321 passed, 9 skipped (196.6 s); exit 0
  - `claudine/`: `just lint` → exit 0
  - `darkmatter/`: `just test shipped_schema_corpus` → 9 passed; exit 0
- **Skipped / pre-existing failures:** none. No cross-OS check: no code
  changed, so there is no OS risk.
- **`claudine` skill:** not updated; Phase 1 changed no behavior or
  architecture.
- **Open ASK items:** none. Rules 1, 2, and 14 were resolved from the
  repository state (see "Dependency check and rulings").

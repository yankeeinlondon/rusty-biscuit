# Phase 1 dependency audit: 2026-10-25-loops-and-flow-control

Audited 2026-10-16 against the worktree `feat/osc-9-4-protocol` as it
stands, **including uncommitted changes**. Paths are relative to `claudine/`
unless they start with `darkmatter/` or `prompts/`. "lib" means
`claudine/lib/src/composition/`, "cli" means `claudine/cli/src/commands/`.
This was a read-only audit. No build or test was run here. Pass counts quoted
from dependency logs come from those logs and were not re-run.

**Worktree caveat.** The review-cycle fixes for 2026-10-24-sequence-containers
are **not committed**. Twenty claudine files are modified (+1054/−93), and
three CLI L1 tests are untracked (`cli/tests/l1/sequence_{deep_cancelled,frozen_graph,referenced_state}.rs`).
Its `review-4.md` reports `ready: true` with no findings. Every container
fact below describes this uncommitted tree.

---

## 1. Delivered-behavior verification

| Claim | In current code? | Evidence |
|---|---|---|
| Loop `fail_fast` still exists | **Yes**, still present. Removing it is this feature's work | Parsed at `lib/looping/config.rs:98-107`. Env read via `resolve_fail_fast_from_env` at `config.rs:22`. Field `LoopConfig.fail_fast` at `lib/types.rs:59`. Runtime option `LoopExecutionOptions.fail_fast` at `lib/looping/types.rs:29`. Engine uses it at `lib/looping/engine.rs:96,334,401,603,735-744`. Still accepted in `LOOP_ITERATION_KEYS` (`lib/lifecycle/parse.rs:71-79`) and in the schema (`schemas/lifecycle.yaml:134`). CLI plumbing is `cli/compose/prep.rs:966,1093`. `cli/wrap/sequence/jit.rs:150` (`step_owned_env(_effective_fail_fast)`) is dead plumbing that ignores its argument |
| Predictive `is_last` | **Yes**, still present. Removing it is this feature's work | `lib/looping/engine.rs:284-306` evaluates the condition before the iteration (`!evaluate_condition(...)`). Described in `docs/topics/flow-control/looping.md:247` as planned for removal. **Coupling:** the rate-limit policy only runs `if !is_last` (`engine.rs:423`), so taking out the prediction changes when the rate-limit pause can fire (see §5) |
| Cap exhaustion | Present, but differs from the spec | On the last pass the gate applies `action` (`engine.rs:481` sets `Continue(next_frontmatter)`). `should_continue_after_cap` then evaluates the condition a **second** time, against the mutated state (`engine.rs:496-522, 925-940`). The spec says no action runs after the last pass and the condition is evaluated once. This feature fixes it |
| Task `setup:`/`teardown:` grammar removed | **Yes, at the grammar.** Dead plumbing remains | Both are rejected with replacement hints at `lib/sequence/grammar.rs:399-403, 446-456`, called from `normalize.rs:164` and `preflight/mod.rs:2171`. `allowed_task_options` (`lib/sequence/reserved.rs`) excludes them, so `PreflightTask.setup/teardown` (`preflight/shape.rs:48,50`, filled at `preflight/mod.rs:871-872`) is always `None` outside the test fixture `preflight/runtime_fixture.rs:67`. Still present: `LifecycleScope::TaskSetup/TaskTeardown` (`lifecycle/bindings.rs:74-126`), `parse_task_action_stack*` (`lifecycle/mod.rs:116-160`, `parse.rs:173-190`), prewarm (`lifecycle/prewarm/sequence.rs:215-221`), task stage code (`sequence/task/mod.rs:553-623, 1046-1051`), and the comment at `lifecycle/actions.rs:6`. This is cleanup only, not a gap |
| Container lifecycle runner | **Yes** | `ContainerRunner` is at `lib/lifecycle/runtime.rs:498-669`, wrapping `LifecycleRunGuard` (`lib/lifecycle/mod.rs:499-953`). It runs `initialize`, `start`, `success`, `failure`, `blocked`, `cancelled`, and `finalize`. `Loop` is routed as `Continue` (`runtime.rs:663`) but **no container runs a loop gate yet**. Decisions are `Continue`, `Skip`, `Failure`, `Finalize`, `Finished{failed}`, `Retry{delay,overlay}`, `Proxy`, `Abort`, and `SetupRecoveryUnsupported` (`runtime.rs:460-480`). Retry keeps `initialize`, resets the guard (`reset_for_next_iteration`, `mod.rs:744`), and advances `attempt`/`budget`. Users: nested sequence `lib/sequence/task/sequence.rs:32-33`, group `task/group.rs:76`, root sequence `cli/wrap/sequence/container.rs:31` (`RootRun`). Documents use the guard directly (`looping/engine.rs:113`, `cli/wrap/composition/pipeline.rs:1765`, `cli/compose/prep.rs:1918`, `cli/wrap/wrapper_stages.rs:602`) |
| `seq`/`group`/`loop` lexical roots | **Yes** | `lib/lifecycle/bindings/containers.rs` holds `SequenceBinding::{standalone,container,step}` (13-48), `GroupBinding`, and `LegOutcome {Succeeded,Failed,Cancelled,Skipped}` (52-133). `event_value` fills legs for terminal events and `Loop` (`containers.rs:104`). The `loop` root is `EnclosingLoop::{Iteration,Pending,Outside}` (`lifecycle/bindings.rs:256-293`). The roots are reserved by `ROOT_OVERLAY_KEYS` (`sequence/reserved.rs:30`) and treated as late shell roots (`preflight/mod.rs:92`, `cli/wrap/sequence/jit.rs:311`). `LoopAmbient` (`looping/expression.rs:19-29`) has `last_output: String`, so it cannot yet carry an array or dictionary. A document's `initialize` runs with `loop` set to Outside (`engine.rs:128-130`), whereas the spec wants count 1 at entry |
| Group `fail_fast`, `cancelled`, sibling cancellation | **Yes** | Group-only `fail_fast` is parsed at `preflight/mod.rs:1140-1144` and rejected elsewhere (`grammar.rs:399`, `sequence/mod.rs:121-123`, `source.rs:111`, `formal.rs:65`). `cancelled` is parsed at `lifecycle/parse.rs:110-125`, and null is rejected. Only `stop` is allowed there (`actions.rs:240-247`). `CancellationCause::{Interrupt,SiblingFailure}` and the token tree are in `lifecycle/cancellation.rs:9-114`. Siblings are cancelled at `task/group.rs:402-410, 476-483`, and admission stops on `fail_fast` (`group.rs:421`) |
| Shell template approval | **Yes** | `ShellPreApprovals {exact, templates}` is at `darkmatter/lib/src/markdown/compose/shell_expansion/template/approval.rs:23-55`. `PreflightGraph.shell_authorization: Arc<Mutex<Option<(ShellPreApprovals, Option<PathBuf>)>>>` is at `lib/sequence/preflight/mod.rs:123,133`, with accessor `:286`, plus `shell_commands` (resolved bytes) and `template_candidates` (`:141,144`). `TaskPosition::First/Later` is at `preflight/shape.rs:99`, and a later task is template-only (test `preflight/tests.rs:1805`). The per-task recheck is `PreflightApproval::verify` (`preflight/reapproval.rs:83-105`, error `SequenceShellApprovalChanged`). Lifecycle templates are in `lifecycle/shell_template.rs`. Graph approval is `approve_preflight_graph` (`cli/wrap/sequence/mod.rs:71-174`) feeding `resolve_graph_shell_approvals` (`lib/preflight.rs:161`). Produced values are template-approved (`error/mod.rs:1900-1921` text) |
| Lifecycle grammar types | **Yes** | `LifecycleNode`/`LifecycleConditional` are at `lifecycle/actions.rs:42-67`. Walkers are `for_each_action(_mut)` (`actions.rs:76-98`) and `validate.rs::walk_stack`. The maximum conditional depth is 5 (`parse.rs:66`) |
| Last-item placement rule | **Yes**, for every control directive | `parse_list` (`parse.rs:355-382`) marks any `is_lifecycle_control()` action as terminal, and a following item raises `LifecycleActionOrder`. Bundles allow only one control verb (`parse.rs:727-732`). The `prep` exception still has to be added here |
| Action enumeration and the "planned" record | `break` and `prep` are **only** recorded as planned | Schema: `schemas/action.yaml:75-85, 253-258, 415-420` and `schemas/lifecycle.yaml:164-169` describe `break` (null, `[]`, string, `{reason, code: 0..255, no_error}`) and `prep` (string, `{target, with, no_error}`) as "planned … not yet accepted by the parser". Parity test: `lib/tests/l1/claudine_schemas.rs:372` `PLANNED = ["break","prep"]`, asserted at `:389-430`, with the cancelled-forbidden list at `:1203-1214`. Parser inventory: `signatures.rs:245-250, 310` (control verbs `stop skip error proxy retry resume defer`). The same list is duplicated at `parse.rs:958-962` (`is_control_verb`) and `source_map.rs:263` |
| `defer` | Parsed, not runnable | Parsed in `action_shape.rs:131-137, 547`. Refused at runtime by `ContainerRunner` (`runtime.rs:654`), the loop gate (`engine.rs:703-708`), and `RootRun` (`container.rs:72-76`). The schema calls it planned |
| Are `break`/`prep` parsed anywhere? | **No** | No `Break`/`Prep` variant exists in `LifecycleControlAction` (`actions.rs:152-213`), `StackControl` (`executor.rs:96-160`), or `ControlDispatch` (`control.rs:26-78`). `- break` and `- prep` fail as `LifecycleUnknownVerb` (`parse.rs:648-649, 711-714`). `lifecycle/tests/grammar.rs:889-899` accepts either `LifecycleActionPlacement` or `UnknownVerb` under `cancelled`, so it must be tightened to placement only once the verbs exist |

## 2. Inventory

### `lib/looping/`

- `config.rs`: `resolve_loop_config(&ResolvedCompositionSource) -> Option<LoopConfig>` (67), `extract_control_variables` (149), `reject_unknown_loop_keys` (278) with hard-coded `suggest_loop_key` (296), `parse_condition` (307), and env readers for `CLAUDINE_FAIL_FAST`/`FAIL_FAST`, `CLAUDINE_MAX_ITERATIONS`, and `CLAUDINE_PAUSE_RESET_MARGIN` (22-56). It reads authored frontmatter only. `max` must be a JSON unsigned integer (`dsl.rs:218-234`), so a **templated `max: "{{ max }}"` is rejected** today. `while`/`until` take any string, **including `""`** (`dsl.rs:197-208`).
- `LoopConfig` (`lib/types.rs:51-64`) has `condition`, `actions`, `max_iterations`, `fail_fast`, and `on_rate_limit`. `OnRateLimit {Pause(default), Abort, Continue}` is at `types.rs:74-100`.
- `types.rs`: `LoopExecutionOptions {max_iterations, fail_fast, on_rate_limit, interrupt_check, pause_reset_margin}` (25-47), `LoopIterationContext {iteration, frontmatter, ambient}` (50), `LoopIterationOutput {output: String, exit_code, error, terminal_signal, rate_limit, exit_reason, provider_id, model_id, handoff, context}` (89), and `LoopExecutionResult {final_exit_code, final_frontmatter, iteration_count, last_output, error, handoff}` (209).
- `engine.rs`: `execute_loop_with_lifecycle<E: FnMut(LoopIterationContext, &mut LifecycleRunGuard) -> Result<LoopIterationOutput>>` (60). It holds `DEFAULT_MAX_ITERATIONS = 100` (25), `run_loop_gate` (gate concerns, then condition, then staged actions; it refuses `retry`/`resume`/`proxy`/`defer` at runtime, 698-724), `decide_rate_limit_action` (843), `interruptible_sleep` (891), `apply_actions` (913), and `should_continue_after_cap` (925). Also in the module: `expression.rs` (`LoopAmbient`, `LoopExpressionLookup`, `evaluate_condition`), `actions.rs` (atomic `ActionStaging`), `seed.rs` (`build_loop_seed_*`), and `dsl.rs`.
- **The loop runs for compose only.** The path is `cli/compose/prep.rs::prepare_and_run_active_document` (494), then `execute_loop_or_single` (1472), then `build_and_run_loop` (983; skipped when `--dry-run`, 1602), then `cli/compose/loop_run.rs::run_loop_with_overrides` (178), then the engine. Each iteration is an `execute_composition_attempt` (`cli/wrap/composition/mod.rs:177`) under the shared guard.

### `lib/lifecycle/`

- `actions.rs`: `LifecycleActionKind {LifecycleControl, Communication, Shell, SideEffect, RuntimeSet, ExpressionFunction}` (128). `LifecycleControlAction {Stop, Skip, Error, Proxy{target,with}, Retry{max_attempts,backoff,delay,with}, Resume{message,max_attempts}, Defer{delay,reason}}` (152). `is_valid_for(signal)` (240) allows only `Stop` in `cancelled` and `Skip` only in `initialize`. `WithOverlay`/`WithValue` (264+).
- `parse.rs`: `parse_lifecycle_config_with` (85), `StackParser::{parse_stack, parse_loop_concerns (287), parse_list (355), parse_item, parse_body, parse_dictionary, parse_bare_verb (623), parse_bundle (676), parse_long_form (767)}`, `check_placement` (906), `canonical_rank` (939), `refuse_initialize_shells` (151), and `parse_with` (only for `proxy`/`retry`, 829-838).
- `action_shape.rs`: `parse_short_form` (8), `classify_positional_value` (20; null or `[]` means 0 args, a scalar is 1, an array is N, an object is an error), `validate_positional_arity_and_build` (81), `build_action_from_params`/`parse_lifecycle_control_long` (~380-520), and `did_you_mean_verb` (Levenshtein, 300).
- `signatures.rs`: `is_known_lifecycle_verb` (231), `all_lifecycle_verbs` (292), `side_effect_signature` (157), and `expression_function_signature` (274).
- `executor.rs`: `StackControl` (96), `LifecycleEventOutcome {control, action_error, evaluation_error}` (165), `StackExecutionContext` (575-664; fields `group`, `seq`, `loop_values`, `runtime_state`, `live_frontmatter`, `shell_runner`, `file_resolution_context`), and the private `ActionStep {Continue, Control, Errored, EvaluationErrored}` (667). The node walk is `execute_stack`/`execute_nodes` (1144-1260); the first control ends the event. `prep` needs a new non-terminating step kind here.
- `control.rs`: `ControlDispatch` (26), `decide_control` (102), `MAX_PROXY_HOPS = 16` (207), `proxy_path_identity` (216), `proxy_handoff_allowed` (257), and `resolve_proxy_target` (291).
- `runtime.rs`: `decide_lifecycle_transition` (354) for documents, the `LifecycleCatchProtocol` (149-300), and `ContainerRunner` (above).
- `validate.rs`: `validate_no_nested_spans_in_literals` and the shared `walk_stack` walker. Its control-action match is at ~192.
- **How signals flow.** The executor evaluates a control into `StackControl`. Documents route it through `decide_control`/`decide_lifecycle_transition` into the harness attempt loop (`cli/wrap/harness_orch/loop_control.rs:142 run_harness_loop`, `loop_control/control_dispatch.rs:50 dispatch_terminal_control`). `retry` re-enters preflight or the provider. `resume` needs a session. `proxy` surfaces as `SurfacedHandoff` for the active-document coordinator. `skip` defuses the guard. `stop` ends the event. Containers route it through `ContainerRunner::record_outcome`, where `resume` becomes `Abort(ResumeWithoutSession)`, `retry`/`proxy` from `initialize`/`blocked` become `SetupRecoveryUnsupported`, and `retry`/`proxy` elsewhere become `Retry`/`Proxy` decisions.

### Sequence normalization, preflight, and task execution (`lib/sequence/`)

- Grammar and normalization: `grammar.rs::validate_task_grammar` (398), `CONTAINER_EVENT_KEYS` (390), `normalize.rs::normalize_plan`, `model.rs` (`SequencePlan`, `SequenceStep`, `StepState`, `SequenceStepOverlay`, `OutputEntry`, `RuntimeMutation`), `reserved.rs` (`EXECUTABLE_KEYS`, `TASK_OPTION_KEYS`, `allowed_task_options`; `when` and `loop` are absent), and `sequence/mod.rs::resolve_sequence_plan_with` (111). No rejection of a root-level `loop:` on a sequence document was found. It appears to be silently ignored, which needs confirming by a test.
- Preflight: `preflight/mod.rs` covers `PreflightGraph` (127), `build_preflight_graph*` (400-450), `prepare_nested_sequence`, group parsing with the **group `loop` rejection** (1127-1135), and task expansion (~860-880). `shape.rs` has `PreflightTask`, `PreflightAction {Prompt, Shell, SideEffect, Sequence, Group}`, and `TaskPosition`. `reapproval.rs` and `containers.rs` round it out.
- Task execution: `task/mod.rs` holds `PromptTaskRunner` (268; `is_dry_run`, `validate`, `run`, `open_member_run`, `refresh_member_run`, `open_leg_run`, `open_group_run`), `PromptTaskRequest` (181), `TaskExecution` (348), `run` (427), `run_primary` (699), `run_prompt_request` (780), `run_shell` (816), `run_side_effect` (957), and `command_timeout` (1255). The nested sequence lives in `task/sequence.rs::run_sequence` (19) and `run_sequence_attempts` (39). The group lives in `task/group.rs::run_group_container` (59), `run_parallel` (337), and `fold_parallel_results` (502). `runtime_state.rs` provides `RuntimeState` with `append_output_entry` (213), `clear_outputs` (98), `replace_sequence_scope` (103), `snapshot` (137), and `set_batch` (173).

### CLI sequence orchestration (`cli/wrap/sequence/`)

`mod.rs::execute_sequence` (246) and `execute_sequence_inner` (326). `phase1c.rs::run_phase_1c_with_schema` (62) does sequence-wide schema and approval. `iterate.rs::run_sequence_steps` (145), `run_one_step` (308), and `run_step_proxy_loop` (749). `task_run.rs::WrapperPromptRunner` (504; `run` at 620). `container.rs::RootRun` (11; `event` at 35, `retry` at 99 calls `clear_outputs`), `mask_body_events` (114), and `run_proxy` (122). `jit.rs` composes each step (`compose_step`, `step_set_overrides`, `sequence_shell_roots`).

`cli/wrap/harness_orch/loop_control*` is the **provider-attempt** loop (retry, resume, proxy, and catch protocol), not iteration. Its files: `coordinator.rs` (the active-document owner), `control_dispatch.rs`, `error_routing.rs`, `lifecycle_events.rs`, `proxy.rs`, `requeue.rs` (dead until `defer` lands), and `target_launch.rs`.

## 3. Seams

| Concern | Seam |
|---|---|
| **Primitive invocation** | Document from compose: `prepare_and_run_active_document` (`cli/compose/prep.rs:494`), which loops. Document as a task: `PromptTaskRunner::run` → `WrapperPromptRunner::run` (`cli/wrap/sequence/task_run.rs:620`) → `jit::compose_step` + `execute_composition_request_inner` (`cli/wrap/composition/mod.rs:154`), a **single attempt**. Shell task: `TaskExecution::run_shell`. Side effect: `run_side_effect` → `StackExecutionContext::dispatch_task_side_effect` (`executor.rs:737`). Nested sequence: `TaskExecution::run_sequence`. Group: `TaskExecution::run_group_container`. Root sequence: `execute_sequence_inner` + `RootRun`. A shared target runner for `prep` has to unify these entry points without calling the CLI recursively. The closest existing reuse is `container.rs::run_proxy` and `iterate.rs::run_step_proxy_loop`, which call `prepare_and_run_active_document` in-process |
| **Lifecycle ownership** | Document: `LifecycleRunGuard` owned by the loop engine (`engine.rs:113`, `initialize` once at 115-160) or the harness (`pipeline.rs:1765`). In a sequence step the document's own events run inside `execute_composition_request_inner`. For a body-only step the sequence masks the document's events (`mask_body_events`, which deliberately keeps `loop`, `container.rs:116`). Containers: `ContainerRunner` per invocation (nested sequence `task/sequence.rs:32`, group `task/group.rs:76`, root `RootRun::new`). Terminal exclusivity and exactly-once `finalize` are enforced by `record_event_emission` (`mod.rs:612-649`) |
| **Approval inventory** | `PreflightGraph` (`shell_commands`, `template_candidates`, `prompt_references`, `prompt_documents`, `shell_authorization`). The front-loaded pass is `approve_preflight_graph` (`cli/wrap/sequence/mod.rs:71`), with closed approvals after the first launch (`approval_options.closed_approvals`, `mod.rs:100-103`). Nested-graph verification is `TaskExecution::verify_nested_graph` (`task/sequence.rs:323`). Document loops use `audit_loop_iteration_template`/`frozen_loop_approvals` (`cli/compose/prep.rs:1773, 1795`). Statically known `prep` targets would be added to `prompt_references`/`prompt_documents` in graph discovery |
| **Cancellation** | A thread-local `CancellationToken` tree (`lifecycle/cancellation.rs:44-114`): `current()`, `child()`, `enter()`, `cause()`, `requested()`, plus `CleanupScope`. Containers poll `TaskExecution::interrupted()` and call `ContainerRunner::cancel`. Documents use `LifecycleRunGuard::cancel_if_requested` (`runtime.rs:714`) |
| **Deadlines** | **No invocation-wide or remaining-deadline type exists.** Wall-clock `timeout`/`step_timeout`/`stall_timeout` are per composition run, copied from `SharedComposeArgs` into each `CompositionExecutionRequest` (`task_run.rs:732-734`; `docs/topics/timeouts.md:32-55, 359`). A shell task's `timeout:` is `TaskExecution::command_timeout` (`task/mod.rs:1255`). `RunLedger::command_started`/`step_started` (`coordinator/invocation.rs:312-319`) are the only anchors. Passing the remaining deadline to `prep` is **new work**, to settle in the Wave 2 spike |
| **Invocation chains** | `RunLedger {chain, approval_cache, command_started, step_started, transitions}` (`coordinator/invocation.rs:262`) records `approve_hop`/`record_proxy`, which are **append-only with no release**. `commit_proxy` is at `coordinator/commit.rs:85`. Hop limit is `MAX_PROXY_HOPS = 16`, with a cycle test by lexical identity or canonical path (`control.rs:207-269`). Note that `RunLedger::contains_document` uses exact equality, which differs from `proxy_handoff_allowed`. Each sequence task gets its own ledger (`task_run.rs:697-700`), so the chain is already per branch. `prep` needs push/pop of active ancestry, shared with `proxy` |
| **Parent result publication** | Sequence: `RuntimeState::append_output_entry` per child (`task/sequence.rs:197`); after `Finished{failed:false}` the nested sequence returns `with_group_output(runtime.outputs_value())` (`task/sequence.rs:246-250`). Group: `fold_parallel_results` builds `GroupBinding` and returns `with_group_output(entries)` (`group.rs:502-580`). Retry calls `runtime.clear_outputs()` in all three container drivers (`task/sequence.rs:263`, `container.rs:101`, and group retry). A looping container instead needs per-pass checkpoints of the output slice and the mutation layer |

## 4. Why a looping document runs once as a sequence step (spec §3)

1. The loop engine is reached only through `execute_loop_or_single` → `build_and_run_loop` (`cli/compose/prep.rs:1472, 1602-1626`), and only from `prepare_and_run_active_document`.
2. A `prompt:` task (sequence step or group leg) runs through `WrapperPromptRunner::run` (`cli/wrap/sequence/task_run.rs:620-830`). That builds a `CompositionExecutionRequest` with `sequence: true` and calls `execute_composition_request_inner` (`cli/wrap/composition/mod.rs:154` → `pipeline.rs:208`), which runs **one** attempt. It never calls `resolve_loop_config` or the engine. The document's `loop:` key is composed and then ignored, and the attempt reports success.
3. Body-only steps follow the same single-attempt path (`TaskExecution::run_prompt_request`, used for a nested sequence's body steps at `task/sequence.rs:182-198`). `mask_body_events` keeps `loop` (`container.rs:116`), but nothing reads it.
4. Asymmetry: a document **reached by `proxy`** inside a step does loop, because `run_step_proxy_loop`/`run_proxy` call `prepare_and_run_active_document` (`iterate.rs:803`, `container.rs:164`).
5. Shipped impact today: `prompts/review-loop.md` calls the looping `prompts/_implement/implement-plan.md` (`loop:` at line 91) as a step, so only phase 1 runs.

The fix seam is to route `WrapperPromptRunner::run` (or a new shared document runner under it) through the same loop-or-single decision. Avoid recursion through `prepare_and_run_active_document`'s CLI coordinator state where it is not needed.

## 5. `on_rate_limit` compatibility inventory

**Behavior.** After each completed iteration that is not predicted to be last, if the provider's stream summary says `rate_limit.is_throttled == true`, the policy applies (`engine.rs:420-445, 843-889`):

- `pause` (the default) sleeps until `reset_at` plus 5 s. The margin can be overridden with `CLAUDINE_PAUSE_RESET_MARGIN`, and the sleep can be interrupted. With no usable reset time it falls back to `abort`.
- `abort` raises `LoopRateLimited` and exits 75 (`EX_TEMPFAIL`, `cli/compose/prep.rs:1635-1645`, `loop_run.rs:56`).
- `continue` proceeds immediately.

Precedence is CLI `--on-rate-limit`, then `loop.on_rate_limit`, then `pause`. The rate-limit signal comes from the provider stream (`IterationSummarySignals`, `runtime.rs:671-709`; `harness_orch/attempt.rs:335`). **No rate-limit recovery exists outside the document loop** (single runs and containers have none).

**Places where it appears.**

| Kind | Location |
|---|---|
| Parse | `lib/looping/config.rs:108-121` (string only), the suggestion at `:302`, `LOOP_ITERATION_KEYS` at `lifecycle/parse.rs:78`, and the type at `lib/types.rs:63-100` |
| Engine | `lib/looping/engine.rs:97-100, 420-445, 843-910`, plus `types.rs:31-47` |
| CLI | `--on-rate-limit` in `SharedComposeArgs` (`cli/compose/mod.rs:209-218, 241-262`), so `compose`, `inline-compose`, and `sequence` all accept it (`cli/sequence.rs:23-24`). Under `sequence` it currently affects only loops reached through `proxy`. Wiring: `build_loop_options` (`cli/compose/prep.rs:958-973`). Completion support: `cli/completion/engine/ownership.rs:50`, `cli/completion/operation_file.rs` |
| Schema | `schemas/lifecycle.yaml:135` (`enum(pause, abort, continue)`) |
| Docs | `docs/topics/flow-control/looping.md:51,58`, `docs/topics/timeouts.md:1288-1325`, `docs/topics/frontmatter-properties.md:24,154`, `docs/topics/flow-control/lifecycle.md:1136`, `docs/topics/completions/shell-completions.md:744-779` |
| Tests | `lib/looping/config/tests.rs` (20 refs), `lib/looping/engine/tests/rate_limits.rs` (pause, abort, continue, reset margin), `engine/tests/{iteration_actions,lifecycle_control,gate_ambient,post_checked_timing,mod}.rs` (struct literals), `lib/composition/error/tests.rs`, `cli/tests/l1/loop_cli.rs:637-870` (abort exits 75, pause waits), `cli/tests/l1/completion_ownership.rs:296,391,420`, `cli/compose/prep/tests.rs` |
| Shipped prompts | **None.** No hit under `prompts/`, `claudine/prompts/`, `darkmatter/prompts/`, or frozen fixtures. (The `suppress_subscription_rate_limit` hits in provider data are unrelated.) |

**Assessment.** The feature is **document/provider-only by nature**: a container has no provider stream and no summary to carry a rate-limit trailer. Removing the predictive `is_last` moves the "skip the policy on the last pass" guard (`engine.rs:423`). The natural replacement is to apply the policy only after the condition and cap decide that another pass will run.

**Recommended ruling (needs the author).** Keep `loop.on_rate_limit` and `--on-rate-limit` accepted, for documents only. Reject the key with a typed error on group and sequence `loop:` blocks. Apply it after the continue decision. Add the robustness-matrix column the plan asks for. Until the author rules, preserve the current behavior.

## 6. Loop keys and loose gate verbs (from code)

**Keys accepted under `loop:`.**

- Iteration controls: `while`, `until`, `action`, `actions`, `max`, `fail_fast`, `on_rate_limit` (`parse.rs:71-79`).
- Lifecycle concerns: `gate`, `no_error`, `set`, and any known lifecycle verb (`config.rs:273-276`).
- `when`, `stack`, and `say_first` reach the lifecycle parser, which rejects them with typed errors (`parse.rs:294-307, 581-626`).

**How unknown keys are rejected.**

- The looping parser raises `LoopInvalid("unknown loop.{key} key (did you mean …); valid keys are: while, until, …")` (`config.rs:278-294`). The suggestions come from a **hard-coded** map (`max_iterations`→`max`, `failfast`→`fail_fast`, `whilst`→`while`, `rate_limit`→`on_rate_limit`) with no edit distance, so a typo such as `untill` gets no hint.
- Unknown verbs in the lifecycle parser use the Levenshtein `did_you_mean_verb` over the verbs only (`action_shape.rs:300`).

**Forms.**

- **Loose form** is a root dictionary bundle (`parse_loop_concerns`, `parse.rs:287-336`). Loose keys plus `gate` is an error, even when `gate` is empty. Several verbs expand in canonical order: `set`, then communication verbs, then `shell`, then other verbs by name, then the one directive. When several verbs are present, each must be in short form; long form is allowed only for a single verb. `say` and `speak` together is an error. At most one control verb is allowed.
- **`gate:` form** is a full stack (`parse_stack`). `null`, `[]`, and `{}` are **all accepted as empty** (`parse.rs:270, 568-570`), which conflicts with the plan's matrix (null is expected to be an error, `{}` an error). A string or other scalar is an error.

**Generic short-form argument rule** (`action_shape.rs:20-61`): null or `[]` means 0 args, a scalar means 1 arg, an array means one arg per element, and an object means long form (`{param: …}`) or, inside a multi-verb bundle, an error. Arity is checked against the verb's signature. `no_error` must be a boolean (sibling or inside long form, not both).

| Verb (loose key) | Short-form args | null / `[]` | Long form | Gate behavior |
|---|---|---|---|---|
| `stop` | exactly 0 | **accepted** | `{}` (+`no_error`) | ends the gate stack; the condition still runs |
| `error` | 0..1 (`reason`) | **accepted** | `{reason, no_error}` | fails the loop (`engine.rs:688`) |
| `skip` | 0 | accepted by arity | — | **parse error** (`is_valid_for` allows `initialize` only) |
| `proxy` | exactly 1 (`target`) | error | `{target, with, no_error}` | parses, **refused at runtime** in the gate (`engine.rs:710-723`) |
| `retry` | 0..1 (`max_attempts`) | accepted | `{max_attempts, delay, backoff: fixed\|exponential, with, no_error}` | parses, refused at runtime |
| `resume` | exactly 1 (`message`) | error | `{message, max_attempts, no_error}` | parses, refused at runtime |
| `defer` | exactly 1 (`delay`) | error | `{delay, reason, no_error}` | parses, refused at runtime (`LifecycleDeferNotImplemented`) |
| `say`, `speak`, `effect`, `message`, `notify`, `stderr`, `stdout`, `info`, `warn`, `success` | exactly 1 (`text`) | error | `{text, no_error}` (`message`/`sound`/`route` removed) | runs. A literal `effect` name must be in the catalog (`parse.rs:922-934`). `success` here is the status-line verb, not the event |
| `shell` | exactly 1 (`command`) | error | `{command, on_error, no_error}` | runs (the gate is post-`start`) |
| `set` | mapping only (never bare, `set()`, long form, or `no_error`) | `set: null` is an error | — | validates, but the gate context has `runtime_state: None` (`engine.rs:~781`), so it does **not persist** |
| `no_error` | boolean sibling | — | — | needs at least one verb beside it |
| Side effects (`darkmatter/lib/src/effects/catalog.rs:61`): `set_frontmatter(file,prop,value)`, `merge_frontmatter(file,obj)`, `delete_frontmatter(file,prop)`, `increment_frontmatter(file,prop)`, `decrement_frontmatter(file,prop)`, `append_frontmatter(file,prop,value)`, `prepend_frontmatter(file,prop,value)`, `ensure_file(file,[content])`, `ensure_dir(dir)`, `append_line(file,text)`, `append_jsonl(file,obj)`, `http_post(url,body)` | the signature's required..max | **error** (every one needs at least 1 arg) | named params in signature order; extras rejected | runs |
| Expression functions: the 113 in `darkmatter/docs/schemas/expression-functions.yaml` (e.g. `file_exists`, `frontmatter`, `and`, `or`, `length`, …) | the merged signature (`signatures.rs:274`); variadic `and`/`or` take positional args only | accepted only where the signature requires 0 args | named params, except for variadic functions | run read-only |

Loose keys named `max` and `min` are always taken as iteration controls (`max`) or rejected (`min` is not a loop key). `LOOP_ITERATION_KEYS` filters them before any verb lookup.

## 7. Shared files likely to conflict, with a proposed owner

| File(s) | Touched by | Proposed owner |
|---|---|---|
| `lib/lifecycle/actions.rs`, `executor.rs` (`StackControl`, `ActionStep`), `control.rs`, `runtime.rs` (`ContainerEventDecision`) | grammar, loop engine, break, prep, wiring | **Coordinator** (shared types, patched before Wave 7) |
| `lib/lifecycle/parse.rs`, `action_shape.rs`, `signatures.rs`, `source_map.rs:263`, `validate.rs` (four copies of the control-verb list) | grammar, prep placement | Configuration grammar |
| `schemas/action.yaml`, `schemas/lifecycle.yaml`, `schemas/README.md`, `lib/tests/l1/claudine_schemas.rs` (`PLANNED`), `lib/lifecycle/tests/grammar.rs` | grammar, docs | Configuration grammar |
| `lib/looping/{config,engine,types,expression,dsl}.rs`, `lib/types.rs` (`LoopConfig`, `OnRateLimit`), `lib/lifecycle/bindings.rs` + `bindings/containers.rs` | grammar (config), loop engine, adapters | Loop state machine (config parsing is shared with grammar, so serialize them) |
| `lib/runtime_state.rs` (pass checkpoints), `lib/error/mod.rs` + `error/render/*` (new error variants) | everyone | **Coordinator** |
| `lib/sequence/reserved.rs`, `grammar.rs`, `normalize.rs`, `preflight/mod.rs` (group-loop rejection at 1127, task options at 871, root `when`), `preflight/shape.rs` | grammar, guard binding, prep inventory | Configuration grammar for keys; Task guard binding for evaluation; Prep runner for graph inventory. `preflight/mod.rs` is the hottest file, so the coordinator should arbitrate |
| `lib/sequence/task/mod.rs` | guard binding, dry run, prep | Task guard binding |
| `lib/sequence/task/{sequence,group}.rs`, `cli/wrap/sequence/{container,iterate}.rs` | container adapters, break, retry, dry run | Container adapters |
| `cli/compose/prep.rs`, `cli/compose/loop_run.rs`, `cli/wrap/sequence/task_run.rs` | document adapter, prep | Document adapter |
| `lib/coordinator/{invocation,commit,handoff}.rs` | prep (ancestry release), proxy | Prep target runner |
| `cli/compose/mod.rs` (`SharedComposeArgs`: `--dry-run-shell`), `cli/wrap/sequence/mod.rs`, `report.rs`, `cli/wrap/composition/pipeline.rs:240` (dry-run seam) | dry run, coordinator | Dry run traversal (the coordinator adds the flag field) |
| `cli/wrap/harness_orch/loop_control/control_dispatch.rs`, `lifecycle_events.rs` | break and prep from document events | Returning-call wiring (Wave 8) |
| `cli/tests/l1/main.rs`, `lib/tests/l1/main.rs` (test registration) | every test task | Coordinator |
| `docs/topics/flow-control/*.md`, `.claude/skills/claudine/*`, `README.md` | docs | Current behavior docs |
| `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`, `resolve_ctx.rs` | path contracts only | Path contracts (independent) |

## 8. Dependency gaps versus this feature's own changes

**Dependency gaps** (promised but missing): **none blocking.**

- The sequence-containers review-cycle fixes exist only as **uncommitted** worktree changes. This is a baseline risk, not a contract gap.
- `setup:`/`teardown:` plumbing remains dead behind the grammar rejection. This is cleanup only.
- `RunLedger::contains_document` (exact equality) is inconsistent with `proxy_handoff_allowed` (identity or canonical path). This predates the dependencies. It is noted, not blocking.

**This feature's own changes**, all confirmed absent today:

- Remove loop `fail_fast` (and the `CLAUDINE_FAIL_FAST` env, the CLI plumbing, and the dead `effective_fail_fast`).
- Remove predictive `is_last` and the second cap-time condition evaluation. Stop applying the action after the last pass.
- Accept a templated `max`.
- Make `loop` available in `initialize`.
- Change `LoopAmbient.last_output` from `String` to `Value`.
- Allow container `loop:` (lift the rejection at `preflight/mod.rs:1127`; add `loop` to the sequence task options; enable a root sequence `loop:`).
- Run document loops for prompt tasks (§4).
- Add `break`, `prep`, task `when:`, and a non-terminating control step.
- Add releasable call ancestry.
- Add a remaining-deadline type.
- Add pass checkpoints in `RuntimeState`, replacing `clear_outputs` for loop retry.
- Bind caller values in shell commands. Preflight already merges `caller_setters` (`preflight/mod.rs:455, 727, 951`); whether the runtime renders them empty, as the spec states, needs a characterization test.
- Make dry run skip `shell:` and `side_effect:` (today nothing gates them).
- Make the empty and null `gate:` shapes strict, reject an empty `while`/`until`, and add did-you-mean suggestions for loop keys.

---

## Baseline required to begin Phase 2

- [ ] The sequence-containers worktree delta (20 modified claudine files and three untracked L1 tests) is kept intact, and preferably committed by the author, before any Phase 2 edit. Phase 2 workers must not stash or reset it.
- [ ] The author rules on `on_rate_limit`. Recommended: document-only, rejected on container loops, applied after the continue decision. The ruling goes in `implementation-log.md`, and the matrix column is added.
- [ ] Coverage inventory and path inventory (Wave 1 siblings) are delivered. Matrix cells are adjusted for current `gate: null|{}` acceptance, empty-condition acceptance, and templated-`max` rejection.
- [ ] The coordinator reserves the shared-type changes (`StackControl`/`LifecycleControlAction` `Break`/`Prep`, a call-returning `ActionStep`, `ContainerEventDecision` break/loop outcomes, `LoopAmbient.last_output: Value`, `RuntimeState` pass checkpoints, the `--dry-run-shell` field, and new `CompositionError` variants) and assigns file owners per §7.
- [ ] The Wave 2 spike decides the deadline carrier (no type exists) and the active-ancestry push/pop design over `RunLedger`.
- [ ] Before Phase 2 starts, confirm a green local baseline (`just test` and `just lint` in `claudine/`) on the current tree. This audit did not run them.

## Verdict

**Dependency gate: PASS.** Every contract the plan names is present in code:

- the lifecycle grammar (nodes, conditionals, placement, and the planned-verb enumeration in schemas and parity tests)
- shell template approval (exact versus template, `TaskPosition`, graph authorization)
- sequence containers (`ContainerRunner`/`LifecycleRunGuard` events, `cancelled`, group `fail_fast`, sibling cancellation, and the `seq`/`group`/`loop` roots)

Loop `fail_fast`, predictive `is_last`, and the single-run prompt task are this feature's own changes, not gaps. Phase 2 may start once the baseline checklist above is satisfied, in particular the uncommitted container delta and the `on_rate_limit` ruling.

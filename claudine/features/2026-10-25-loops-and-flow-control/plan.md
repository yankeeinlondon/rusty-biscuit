---
kind: plan
area: claudine
created: 2026-10-12
total_phases: 6
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
    - claudine/cli/tests/l1/loop_config_matrix.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/features/2026-10-25-loops-and-flow-control/spike/loop_flow_spike.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/dependency-audit.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/path-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/coverage-inventory.md
    - claudine/features/2026-10-25-loops-and-flow-control/phase-1/matrix-contracts.md
    - claudine/features/2026-10-25-loops-and-flow-control/spike/README.md
skills_files_updated_during_phase_1: []
packages:
    - claudine-cli
---

# Loops and flow control implementation plan

## Work and successful completion

Implement 2026-10-25-loops-and-flow-control after its lifecycle, approval, and container dependencies. Replace document-only repetition with one loop mechanism used by documents, groups, and sequences. Add nearest-loop `break`, returning `prep`, task `when:`, caller-value binding, and a preview path that renders shell and side-effect tasks without executing them. Correct Darkmatter's path-building contracts, migrate the review workflow, and update current documentation and schemas alongside behavior.

Success means all 14 specification acceptance criteria have observable coverage: the counting document runs three times through every entry point; event order, nested state, retry rollback, output publication, and failure precedence agree across primitives; helper calls return without leaking caller state; guards skip without hiding invalid configuration; and dry run leaves a scratch Git index and side-effect files untouched. The migrated review workflow must distinguish ready, recurrence, invalid or stale reviews, and exhaustion. Claudine and Darkmatter L1 and lint checks pass; affected terminal tests pass without gaining focus. No real provider, developer Git index, new CI cell, commit, or lifecycle-directory move is part of this work.

Each wave is a dependency barrier, numbered uniquely across the plan. Tasks within an explicitly parallel wave may be assigned to subagents after their interfaces are agreed; tasks in a serial wave execute in listed order. Phase checkpoints must pass before the next phase starts. An implementation coordinator owns shared-file changes and integrates each wave before releasing the next. The planning step does not run implementation spikes or tests.

## Phase 1: Establish contracts and readiness

### Necessary Rules

The specification reports no open questions. Preserve its decisions; the following rules resolve sequencing and identify the remaining compatibility ruling rather than reopening decided semantics.

1. **Dependency readiness is a gate.** Verify delivered behavior from 2026-10-20-lifecycle-grammar, 2026-10-23-shell-template-approval, and 2026-10-24-sequence-containers, not just the existence of their specs. The current checkout still has loop `fail_fast`, predictive `is_last`, and task teardown code; do not assume the future container baseline exists. If prerequisites are absent, record the missing contracts and stop dependent implementation. Also verify the action enumeration/schema surface from 2026-10-21-claudine-schemas is available; do not implement that feature wholesale here.
2. **Use current input spelling.** Until 2026-10-26-task-inputs-and-providers lands, task inputs remain `params:` in code, fixtures, and shipped prompts. Lifecycle `prep.with` remains `with:`. Do not add per-task providers or implement `defer`.
3. **One repetition mechanism, distinct bodies.** Reuse primitive-specific execution and container lifecycle guards. No container-to-document rewriting and no recursive CLI invocation for `prep`. Freeze the task graph and loop configuration once; retain authored value/expression sources for existing preparation boundaries. Produced values stay data.
4. **Post-run semantics are decided.** Condition evaluation follows terminal cleanup and gate execution. `is_last` means `count == max` only. Test the condition before classifying cap exhaustion. No performance spike is needed: the spec identifies a semantic defect, not a measured performance problem. Any wider measurement needs an author ruling before adding it.
5. **Recovery and break precedence are decided.** Retry restores only the current pass's output slice and runtime mutation layer, then applies retry inputs. Completed passes, count, and initialization survive. External effects do not roll back. Unhandled failure, cancellation, and timeout defeat pending break; handled group-leg failures do not. Post-cleanup errors never re-enter completed terminal events or cleanup.
6. **Preview is a separate traversal.** Visit each body once, display future-dependent guards and conditions, do not run lifecycle events or `prep`, and do not mutate runtime output ledgers. `--dry-run-shell` requires `--dry-run`, executes shell tasks once, and never enables side-effect tasks or providers. Composition `$(...)` remains effectful and is disclosed in help.
7. **Path changes are surgical.** Preserve existing shared resolver usage. Change `absolute` to build missing local paths; preserve absolute anchors in `dirname`, including inputs inside a projection root. Keep read/existence contracts and typed probe errors. Load `biscuit-file`, Darkmatter, and OS skills before implementation; use captured `FileReference` context and native fixtures on each OS.
8. **Compatibility ruling required before Phase 2.** Existing `LoopConfig` and the engine expose `on_rate_limit` and pause-policy behavior, but the spec's new loop grammar does not name that key. Inventory current callers and decide with the author whether it remains accepted and, if so, whether it is document/provider-only or applicable to containers. Preserve existing provider rate-limit recovery until this ruling is made; do not silently remove it or invent container behavior. Record the ruling in the implementation log, leaving the finalized spec unchanged.
9. **Strict configuration shapes.** The robustness tables below define proposed handling where the spec does not give a shape-level rule. Confirm empty action/gate/overlay collections and strict duplicate-key handling in Phase 1. Reject duplicate executable-configuration keys before a lossy map conversion, including JSON-family inputs. This is a configuration contract, not a ban on Markdown body content, JSON5 comments, or valid subsequent JSONL records.
10. **Testing and closure.** Use Test Toolkit, nextest, hermetic `CliProcessFixture`, private home/CWD/spool, and recording stub providers. Prompt contracts are opt-in through `prompt-tests`; behavior fixtures live under frozen fixtures. Use `biscuit-hash`/Darkmatter's existing hash path for route hashes. Never run `cargo fmt`, commit, push, or move the feature into `_completed`. Final state is implementation complete, ready for review.

### Wave 1 — parallel inventories

- [x] **Dependency audit**
  - Compare landed APIs and tests with the dependency gate above. Inventory `composition/looping/{config,engine,types}.rs`, `lifecycle/{actions,executor,control,parse,validate,signatures}.rs`, sequence normalization/preflight/task execution, CLI sequence orchestration, and the prerequisite container runner.
  - Record concrete seams for primitive invocation, lifecycle ownership, approval inventory, cancellation/deadlines, invocation chains, and parent result publication. Separate dependency gaps from this feature's changes.
  - Deliver an implementation log with the compatibility ruling, shared-file owners, and the exact baseline required to begin Phase 2.
- [x] **Path inventory**
  - Independently inventory Darkmatter's `expression/functions/mod.rs`, `resolve_ctx.rs`, path-shaping helpers, file-index operations, and existing tests. Record each function's return type, missing-target behavior, URL behavior, and error propagation.
  - Include an absolute caller path inside the repository: current `dirname` uses projected display paths and can still lose that absolute input's anchor. Keep pure filename operations unchanged.
  - Deliver the path contract/fixture table used by Phase 2; this task does not depend on future container APIs.
- [x] **Coverage inventory**
  - Locate existing loop/lifecycle/sequence tests, consolidated test registration, prompt-test recipes, frozen prompts, and `shipped_prompt_route_drift` fixture derivation.
  - Map every numbered acceptance criterion to a planned public assertion and owning task. Verify supported executable configuration formats; include YAML frontmatter and external YAML/JSON/JSON5/JSONL/NDJSON task sources that can carry the changed fields.
  - Establish the format-specific fixture cases and approved absence/null rules below before implementation. Do not extend this matrix to Markdown bodies or expression text grammar.

### Wave 2 — serial control-flow spike

- [x] **Control flow spike**
  - Prerequisite: dependency audit passes. Run one bounded, in-process experiment with fake primitive bodies and the existing lifecycle/runner interfaces.
  - Demonstrate a typed result carrying a loop identity, a suspended event continuing after `prep`, and exactly-once cleanup when a break crosses a container. Include one pending concurrent break and one failed cleanup.
  - Demonstrate current-pass checkpoint restoration without erasing an earlier pass. Determine how the shared target runner passes cancellation, remaining deadline, and branch-local call/handoff ancestry without borrowing caller runtime roots.
  - Deliver the smallest viable interface contracts and trace expectations; keep no alternate production runner. Run once, before the engine and control work it informs. Do not repeat this spike in later phases or turn it into a performance/OS survey.

### Input robustness matrix

These tables cover configuration read by the changed execution paths. Apply each applicable column independently to **YAML** (standalone configuration and Markdown frontmatter), **JSON**, **JSON5**, **JSONL**, and **NDJSON** task-source fixtures. JSONL/NDJSON edits occur within one task record; a valid next record is allowed. Document-only fields use YAML frontmatter; task/configuration columns apply wherever the existing loader exposes those objects. Verify this applicability during Wave 1 and record unsupported entry points rather than adding formats. Review/spec metadata is YAML frontmatter only. Scalar/list/map alternatives retain their existing validated action grammar.

`Error` means a located public execution/preflight failure, no provider or task side effect, and no parent output published. It must never become false, absent, or an empty result. `N/A → Error` means inserting a collection in a scalar field is a whole-field type error. For nested collection cells, edit one nested member or every member, rather than inventing scalar elements for a mapping. Empty strings, empty maps, and empty lists are separate cases even where the table groups their outcomes.

| Shape | `loop` | `loop.until` | `loop.while` | `loop.max` | `loop.action` | `loop.actions` |
|---|---|---|---|---|---|---|
| Control | Valid owner repeats to condition | Valid expression ends run | Valid expression ends run | Positive cap bounds passes | Valid mutation changes next pass | Same alias behavior |
| Absent | Owner runs once | Require `while` | Require `until` | Runtime/env override or 100 | No continuation mutation | No continuation mutation |
| Explicit null | Error | Error | Error | Error | Error | Error |
| Wrong whole type | Error | Error | Error | Error | Error outside accepted forms | Error outside accepted forms |
| One wrong element/member | Error in nested field | N/A → Error | N/A → Error | N/A → Error | Error; no partial staging | Error; no partial staging |
| Every element/member wrong | Error | N/A → Error | N/A → Error | N/A → Error | Error; no mutation | Error; no mutation |
| Empty | `{}` errors: no condition; other shapes error | Empty expression errors | Empty expression errors | Empty value errors | `[]` is explicit no-op; empty string/map errors | Same as `action` |
| Duplicate key | Error | Error | Error | Error | Error | Error |
| Trailing/invalid content | Error | Error | Error | Error | Error | Error |

Additionally test both conditions together, both action aliases together, zero/negative/fractional/unresolved/overflowing caps, and override precedence independently for nested loops. `loop.fail_fast` and root-level `loop.when` are rejected for every present shape, including null and apparently valid values; absence is valid. Any unknown loop key errors with applicable suggestions. The rate-limit compatibility ruling adds an explicit `loop.on_rate_limit` column before parser work if that field remains supported.

| Shape | `loop.gate` | Each accepted loose `loop.<verb>` | Task `when` | Task `loop` placement |
|---|---|---|---|
| Control | Valid actions run post-finalize | Valid verb bundle runs post-finalize | Truthy runs; falsey skips | Inline sequence accepts valid loop |
| Absent | No explicit gate | No action for that verb | Task runs normally | No task-owned repetition |
| Explicit null | Error | Error unless existing verb contract explicitly accepts it | Error | Error |
| Wrong whole type | Error | Error under verb signature | Error | Error |
| One wrong element/member | Error; execute nothing | Error; execute nothing | N/A → Error | Nested loop field error |
| Every element/member wrong | Error | Error | N/A → Error | Error |
| Empty | `[]` is explicit no-op; empty map/string errors | Accepted only if verb signature defines it; otherwise error | Empty expression errors | Empty map errors |
| Duplicate key | Error | Error | Error | Error |
| Trailing/invalid content | Error | Error | Error | Error |

Expand the loose-verb column into one column per accepted verb using the dependency's actual signatures before Phase 2; do not replace that inventory with a guessed list. Mixing loose verbs and `gate` errors even when a collection is empty. `when` at a sequence document root errors. Task-level `loop` on prompt/shell/side-effect/group-reference task options errors; a group's loop belongs on its group object. Preserve inherited lifecycle restrictions, including shell-free initialization and passive validation.

| Shape | `break.reason` | `break.code` | `prep.target` | `prep.with` | Each changed task `params.<key>` binding |
|---|---|---|---|---|---|
| Control | Summary includes reason | Root exits with code; inner loop records it | Target executes and caller continues | Child receives explicit inputs only | Caller value resolves at correct boundary |
| Absent | No reason | 0 | Error | No input overlay | Declared schema decides required/default/optional |
| Explicit null | Error | Error | Error | Error | Preserve null as data only if declared schema permits |
| Wrong whole type | Error | Error | Error | Error | Schema/type error; never empty substitution |
| One wrong element/member | N/A → Error | N/A → Error | N/A → Error | Error for invalid overlay syntax/value form | Error if input schema forbids that member |
| Every element/member wrong | N/A → Error | N/A → Error | N/A → Error | Error, never empty overlay | Error if input schema forbids those members |
| Empty | Empty string is explicit empty reason; collections error | Error | Empty reference errors | `{}` is explicit empty overlay; string/list error | Preserve valid empty data; required file/reference cannot be empty |
| Duplicate key | Error | Error | Error | Error, including nested input keys | Error |
| Trailing/invalid content | Error | Error | Error | Error | Error |

Cover bare/positional/object directive forms separately, code range `0..=255`, non-integers, missing target, and unknown arguments. Expand the task binding column for the real fixture's load-bearing keys (at least caller `agent` and caller `spec`), including provenance and authored-versus-produced values. Arbitrary valid typed overlay data is not an invalid element. No new general input schema or task rename is introduced.

| Shape (YAML metadata) | Spec `implemented` | Spec `review_iterations` | Review `ready` | Review `recurrence` | Review-loop `spec` | Review-loop `agent` | Review-loop `max` |
|---|---|---|---|---|---|---|---|
| Control | Boolean true skips implementation | Counter advances exactly one | Boolean true breaks ready | Boolean true breaks recurrence first | Caller file selects reviewed spec | Input selects stub provider | Positive cap limits reviews |
| Absent | Not true: implementation needed | Before review: zero; after review: error | Error | Error | Required-input error | Required-input error | Authored default 5 |
| Explicit null | Distinct data; not true, implementation needed | Error, including before review | Error | Error | Error | Error | Error |
| Wrong whole type | Preserve spec's `!= true` comparison; record result | Error | Error, including quoted booleans | Error, including quoted booleans | Schema error | Schema error | Error |
| One wrong element | Collection comparison follows existing expression rules | N/A → Error | N/A → Error | N/A → Error | N/A → Error | N/A → Error | N/A → Error |
| Every element wrong | Same existing comparison rules | N/A → Error | N/A → Error | N/A → Error | N/A → Error | N/A → Error | N/A → Error |
| Empty | Existing comparison; no silent coercion to absent | Error | Error | Error | Error | Error | Error |
| Duplicate key | Error | Error | Error | Error | Error | Error | Error |
| Trailing/invalid content | Invalid metadata errors; valid Markdown body allowed | Same | Same | Same | Same | Same | Same |

The `implemented` examples deliberately retain the specified expression semantics rather than inventing a new boolean validator; characterize the actual public decision for non-boolean/collection cells in Phase 1. By contrast `ready` and `recurrence` must be actual booleans. A missing pre-review counter means zero; explicit null must fail as a present non-integer. If the current `frontmatter()` surface conflates them, identify the smallest presence-aware read needed before migration; do not silently accept null as absent.

- [x] **Matrix contracts**
  - Finish all expanded per-field columns and any necessary owner rulings before parsing work. Start from actual tool-produced/accepted fixtures (serialized through existing public format tools and confirmed by Claudine), with a positive control that demonstrably launches, loops, skips, or selects a result.
  - Implement one parameterized matrix traversal per supported format, derived from that fixture with one edit per cell. Assert public status, launch/effect counts, published results, or summaries—not parser return values. Include source location on errors and preserve valid body/record content.
  - Audit for `#[serde(default)]`, absent/null-conflating `Option<T>`, silent `filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()` on load-bearing parses. Use presence-aware deserialization when needed. A review finding reports every other cell in that row and column.

**Checkpoint:** dependency readiness, expanded robustness contracts, accepted compatibility rulings, acceptance mapping, and the one-time spike report are available. All later tasks have named seams and file ownership; no prerequisite feature is silently absorbed.

## Phase 2: Extend configuration and path contracts

### Wave 3 — parallel implementation

- [ ] **Configuration grammar**
  - Prerequisite: Phase 1 checkpoint. Extend loop parsing to every permitted primitive location; parse templated caps for one-time resolution using existing option/env precedence. Reject loop `fail_fast`, invalid/unknown keys, conflicting conditions/aliases, and mixed gate forms with typed, located diagnostics and suggestions.
  - Add `break`/`prep` arguments to shared actions, signatures, validators, action enumeration, and `claudine/schemas/` partials. Permit non-final `prep`; retain last-item placement for unconditional terminating directives. Reject both under `cancelled` and preserve shell-free initialization checks.
  - Parse `when` as a reserved task option, not step data; reject sequence-root `when` and unsupported task loop locations. Retain current task `params` spelling.
  - Deliver typed configuration/control contracts from the spike without executing helpers during validation. Run format matrix tests against public preflight and execution as those seams become available; passive-invalid cases must execute nothing.
- [ ] **Path contracts**
  - Independent owner: Darkmatter functions/resolver tests and corresponding docs/catalog/skill. Reuse the current path-shaping resolver and `FileReference`; never recapture anchors.
  - Make `absolute` return the first local candidate for a missing target and reject HTTP(S). Preserve an absolute input's parent anchor in `dirname`. Resolve file-index inputs before changing names, retaining their parent/provenance. Preserve existing read/existence contracts and typed permission/reference/probe failures.
  - Add table-driven public expression tests for collisions, explicit `./`/`../`, caller provenance, prefixed/absolute references, missing files, outside-repo operation, spaces, URLs, and probe errors. Construct Windows-native paths only on Windows; use existing portable identity helpers for comparisons.
  - Update function comments, expression docs, catalog, relevant README examples, and Darkmatter skill together. Run scoped path tests and appropriate Darkmatter checks; no new resolver or dependency by default.

**Checkpoint:** configuration accepts exactly the new grammar and remains passive; matrix errors are observable through public results. Darkmatter path contracts are verified, including absolute callers under projection roots. Shared configuration/result types are stable for parallel primitive adapters.

## Phase 3: Unify primitive repetition

### Wave 4 — serial engine work

- [ ] **Loop state machine**
  - Build one primitive-neutral repetition mechanism around the existing lifecycle guard, with typed document/string, sequence/array, and group/dictionary pass results. Allocate the nearest-loop frame before `initialize`; expose initial empty result, exit code 0, count 1, and cap-only `is_last`.
  - Implement initialize-once, per-pass preparation/start/terminal/finalize, post-finalize gate, fresh condition reads, condition-before-exhaustion, and continuation-only atomic action staging. Mutate document state or container runtime `set` state as appropriate.
  - Remove predictive condition evaluation and loop-level fail-fast behavior without changing group fail-fast or approved provider recovery. Freeze graph/configuration; keep live authored expression/preparation boundaries and data provenance.
  - Add typed condition-complete, exhausted, and broken outcomes; iteration/source diagnostics for gate/condition/action failures; distinct failure/handoff/cancel/timeout handling. Do not publish or replay terminal cleanup after post-cleanup errors.
  - Deliver engine conformance tests and adapter interface before Wave 5.

### Wave 5 — parallel primitive adapters

- [ ] **Document adapter**
  - Route direct compose, inline-compose, sequence prompt tasks, and group prompt legs through the same document repetition path. The section 3 counting fixture must launch three prompts in each applicable entry point.
  - Keep document lifecycle ownership, template approval audit, live re-read, rate-limit handling as ruled, and raw final-iteration output. Add lifecycle/count/output trace tests.
  - Own document runner and prompt adapter files; request coordinator changes to shared sequence plumbing rather than editing the container owner's files.
- [ ] **Container adapters**
  - Apply the shared mechanism to root/nested sequences and inline/file groups while retaining their own schema, neighbors, concurrency, runtime roots, and aggregate failure rules.
  - Maintain cumulative flat `seq.outputs` separately from the current pass slice; rebuild group leg results and summaries each pass, preserving null for skipped legs. Reset each descendant invocation's initialization and inner loop count on the next outer pass.
  - Thread innermost `loop.*` through owner events, gate, conditions/actions, and descendants. Retain first-task resolved-byte approval and later typed-template evaluation on repeated passes.
  - Own container adapters; coordinator owns shared result/state types. Prove three passes, nested loop shadowing, and no publication until successful cleanup.

### Wave 6 — serial recovery integration

- [ ] **Retry publication**
  - Prerequisite: both adapters integrated. Snapshot current-pass accumulator boundaries and runtime mutation state; restore only those on retry and apply retry `with` overlays afterward. Rebuild group pass results without resetting earlier sequence outputs, cap/count, or initialize.
  - Preserve authored/data and file provenance in restored layers. Record external writes repeating without attempting rollback.
  - Prove second-pass `A`/`B` failure and retry, repeated retries, nested looping containers, group rebuilding, and exactly-once final cleanup. Assert parent publication is absent for failure/cancellation and successful results publish once.
  - Cover final-cap condition success, exhaustion with no last action, atomic staging failure, live file reads, and all loop outcome summaries.

**Checkpoint:** acceptance criteria 1–4 and 14 pass through public execution. Documents and containers share repetition; event traces, pass slices, nested ambient values, approval boundaries, and retry state are stable enough to wire new directives.

## Phase 4: Add control, guards, and preview

### Wave 7 — parallel work with explicit file ownership

- [ ] **Break propagation**
  - Own loop/container outcome routing and lifecycle cleanup integration. Carry the selected nearest-loop identity and reason/code through intermediate containers; stop later actions/events/siblings except cleanup, retaining prior mutations and successful partial results.
  - Cover initialize without launch, terminal events, finalize, and gate; runtime error outside a loop; root-only exit status; child/container publication after cleanup with no synthetic break entry.
  - In concurrent groups, settle admitted and queued legs without break-driven cancellation/admission stops. Choose the first breaking leg in declaration order, independent of completion order. Failure/cancellation/timeout and failed cleanup take precedence.
  - Add deterministic queued-leg and precedence tests using barriers/notifications, not fixed sleeps. Leave lifecycle `prep` wiring to Wave 8 to avoid shared-file edits.
- [ ] **Prep target runner**
  - Own shared target invocation/preparation and approval inventory support, outside the break-routing files. Implement document/group/sequence target calls with isolated runtime roots, output accumulator, and loop boundary; preserve overlay origins and author-relative resolution.
  - Reuse the target runner for calls/handoffs, keeping mixed active ancestry and hop limits per branch. Release returned children so repeated calls are legal; permit a target proxy to complete before returning.
  - Pass cancellation and the remaining caller wall-clock deadline without reset. Preserve disk effects, attach child failure causes, treat child skip as success and nonzero child root-break status as call failure.
  - Inventory statically known targets transitively without running them; runtime-only targets must obey frozen approval. From initialize, prepare the child independently and rerun parent preflight after return, retaining the parent's shell-free guard.
- [ ] **Task guard binding**
  - Own task normalization/evaluation and input binding. Keep the graph structural and frozen but preserve authored templates for task-boundary resolution; make caller setters/frontmatter available without shadowing reserved roots or evaluating produced data again.
  - Evaluate read-only `when` before composition/events/loop entry. False means skipped with no launch/output; expression errors are located failures. Static validation/approval must still inventory false-guard tasks.
  - Freeze concurrent runtime guard inputs at group start even for delayed admission; file reads remain live. Reevaluate guards each outer pass. Test caller values in shell commands and authored step data, missing values, and changed first-task approved bytes.
- [ ] **Dry run traversal**
  - Own CLI flag/help, preview dispatch, and report rendering; do not edit live execution routing owned above. Add `--dry-run-shell` requiring `--dry-run` and clearly disclose mutation and composition expansion effects.
  - Render each loop body once, repetition/conditions unevaluated, task guards displayed, known prep targets listed. Execute no lifecycle, helper, provider, side-effect, loop action, runtime ledger write, or completed-task summary.
  - Render shells with approved bytes or typed templates; report-only empty placeholders never enter runtime accumulators. The opt-in executes shells once only. Side-effect previews show action/arguments and unresolved future templates under both flag combinations.
  - Use existing biscuit-terminal renderables. Verify scratch Git staging, scratch file/runtime set absence, future-dependent previews terminating, and usage errors.

The coordinator reserves shared request/options/result structs and patches them before this wave. If actual prerequisite code puts two tasks in the same orchestration file, move the overlapping wiring to Wave 8; parallel workers deliver their isolated seams and tests only.

### Wave 8 — serial directive integration

- [ ] **Returning call wiring**
  - Connect the target runner to lifecycle execution as a returning action, not an event-ending `proxy` signal. After success, continue the same list and enclosing conditional list. Keep break targeting inside the called invocation only.
  - Integrate failure/finalize call errors without re-entering that event or replacing the original primary error; verify cancellation cleanup and exactly-once finalization across nested calls/handoffs.
  - Exercise all new surfaces together: guard plus nested loop, prep from initialize, repeated prep plus outer loop, target proxy, pending break with handled/unhandled failures, frozen command approval, and preview with helper-containing loops.
  - Complete public directive/task/preview matrix assertions and signatures/schema/runtime parity checks. Correct behavior-changing comments alongside code.

**Checkpoint:** acceptance criteria 5–9 pass, plus path criterion 10 from Phase 2. Trace/public output assertions prove nearest-loop behavior, isolated returning calls, guard snapshots, deadlines, and nonmutating default preview; no synthetic runtime results are used to make tests pass.

## Phase 5: Migrate prompts and current documentation

### Wave 9 — parallel prompt and documentation work

- [ ] **Review prompt migration**
  - Rewrite `prompts/review-loop.md` as the specified looping sequence, using current task `params`. Guard implementation, keep review as a group of one, and capture review counter/path in the group's start layer for the successful state merge.
  - Use presence-aware pre-review counter validation, require a previously absent numbered file, an increment of exactly one, and actual boolean flags. Recurrence wins over ready; error before repair/stage/commit for malformed, missing, stale, or pre-existing files. Never delete a pre-existing review to make rehearsal succeed.
  - Preserve caller-file/space-safe sibling paths and fresh explicit file reads. Keep cap 5 semantics: at most five reviews and repairs; an unverified final repair exhausts. Ensure implementation's own multi-phase loop runs fully as a prompt task.
  - Replace commit initialize `stop` with `skip`. Remove `in_loop` gating from the three specified implementation/review helpers and all shipped callers/frozen routes; retain explicit inputs and standalone repair behavior.
- [ ] **Current behavior docs**
  - Update Claudine README, `looping.md`, `sequences.md`, lifecycle and flow-control/reference pages, CLI/dry-run guidance, and the Claudine skill. Remove implemented-feature planned markers, the loop defect note, and incorrect prep-output append claims; leave `defer` planned.
  - Explain practical examples for every primitive, post-run order, cap-only `is_last`, retry pass restoration, partial results, nearest-loop/root exit codes, disk-only helper return, guard snapshots, template approval, and dry-run limitations.
  - Add Mermaid diagrams for iteration order and break propagation. Explain why opt-out initialization uses `skip` instead of `stop`. Keep topic docs self-contained and free of feature/spec references.
  - Confirm Phase 2 Darkmatter docs/catalog/README/skill reflect missing-target `absolute` and preserved `dirname` anchors. Update dependency docs only if actual crate dependencies change.

### Wave 10 — serial fixture integration

- [ ] **Workflow rehearsals**
  - Derive frozen behavior fixtures from the migrated prompts and preserve their load-bearing schema/loop/body semantics. Refresh route hashes through the existing Darkmatter/biscuit-hash workflow only after checking fixture deltas; include provenance and initialize fixtures.
  - Use recording stub providers, private CWD/home/spool, and scratch Git repositories. Rehearse an unimplemented two-phase plan through one repair/commit then ready, and an already implemented spec becoming ready without implementation/commit launches.
  - Cover recurrence exit 2, both flags true, ready on the last permitted pass, cap exhaustion exit 1, invalid flags/counters/files, existing next-numbered review, caller spec outside prompt directories, and names containing spaces. For the commit behavior, record/stub commit execution so no test needs a signing prompt.
  - Assert no provider launches for empty staged state and no host audio spool. Run `shipped_prompt_contract`, `compose_caller_file_provenance`, `compose_initialize_acceptance`, and `shipped_prompt_route_drift` in their registered tiers/features. Use `just test-prompts` if provided by the landed baseline; otherwise the documented noninteractive nextest `prompt-tests` command.

**Checkpoint:** acceptance criteria 11–13 pass. Prompt routes and frozen copies agree, review freshness/typed metadata are enforced, standalone helper behavior remains usable, and current docs describe implemented behavior.

## Phase 6: Verify and hand off

### Wave 11 — parallel independent validation

- [ ] **Claudine verification**
  - Run `just test` and `just lint` from the exact `claudine/` worktree area; explicitly run prompt-feature contracts, since ordinary L1 does not enable them. Verify all new tests are registered in consolidated binaries and no format/acceptance case is stranded.
  - Run `just test-l2` only for affected real-terminal behavior/routes. Use the shared harness and visibility policy without gaining terminal/browser focus. Keep behavior proofs at L1 wherever a hermetic subprocess or fake body suffices.
  - Review public diagnostics and summaries for raw outputs, loop iteration/reason/code, skipped versus broken status, and source locations. Audit comments changed by behavior, schema/signature parity, and absence of recursive CLI prep.
- [ ] **Darkmatter verification**
  - Independently run `just test` and `just lint` in the exact `darkmatter/` area. Check affected direct dependents according to the path-function blast radius, without a workspace-wide test run.
  - Confirm every inventoried path operation retains its documented type, missing/read contract, captured context, and URL/probe-error handling. Validate existing/missing examples against public expressions.
- [ ] **Portable evidence**
  - Load the OS skill and use existing environment-specific evidence/host procedures for macOS, Linux, native Windows, and WSL2. Reuse qualifying evidence; record which required checks actually ran and any unavailable evidence. Do not claim cross-compilation proves native runtime behavior.
  - Keep portable stub providers, native path fixtures, archived runtime fixture roots, noninteractive command builders, deterministic concurrency, and no focus changes. Where another OS exposes a trap, fix it and update the OS skill in the same change.
  - Use existing CI scope only; no new matrix/gate or speculative full run. Do not initiate a push as part of implementation closure.

**Final checkpoint:** all 14 acceptance criteria and every applicable robustness cell are accounted for by passing public-result tests, or a specific missing-evidence limitation is reported without claiming completion of that check. Claudine/Darkmatter required local checks and affected opt-in tests pass. Save the implementation log with rulings, spike result, deviations, verification commands/results, and remaining review risks; mark completed plan tasks. Leave the specification unchanged and the feature in its active directory, with implementation complete, ready for review.

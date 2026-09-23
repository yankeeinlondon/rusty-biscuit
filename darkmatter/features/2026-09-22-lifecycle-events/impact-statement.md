---
prompt: |-
    <skill name="claudine">
    ::file ^.claude/skills/claudine/SKILL.md
    </skill>
    <skill>
    ::file ^.claude/skills/darkmatter/SKILL.md
    </skill>
    
    ## Context
    
    Currently we have a rich lifecycle event model in Claudine but the more I think about it the more I'm certain that Darkmatter is the better owner of this lifecycle system. Then Darkmatter would own core events but would expose an API for callers to add in their own events if the base events provided didn't suffice.

    What has prevented me so far on doing this is the **scale** of the change which is involved but in reality I don't have that good a picture of the scale. 

    ## Task
     
    Your job is to evaluate the change that is required:

    - try to find a good way to categorize the change
    - using this categorization model to talk about the scale of the change work
    - identify any "de-risking" measures you can think of
hash: 98244c66275c9a7f-01e72f7c3f473118
last_updated: 2026-09-19
---
# Impact assessment: moving document lifecycle ownership to Darkmatter

Darkmatter is a plausible owner of the reusable document lifecycle engine. This is a **medium-to-large extraction and API design project**, rather than a wholesale rewrite of Claudine. The expression evaluator, subtree composition, effects engine, and file-resolution machinery already live in Darkmatter. The difficult work is separating lifecycle semantics from the host that launches agents, supplies diagnostics, and implements recovery.

The recommended scope is to move parsing, validation, event-stack evaluation, and reusable transition rules into Darkmatter, while retaining provider execution and application policy in Claudine. Custom events need a deliberate registration contract; replacing the event enum with strings would leave much of the current behavior hard-coded elsewhere.

This assessment reflects the working tree inspected on September 19, 2026. Proposed ownership and effort estimates below are recommendations, not an approved implementation design.

## Scope: two systems currently called lifecycle

Claudine has two distinct event systems:

| System                                                                                         | Current authority                                                                | Recommended treatment                                                                       |
|------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| Document lifecycle: `initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, `loop` | `claudine/lib/src/composition/lifecycle/`, with library and CLI orchestration    | Extract the reusable engine and preserve Claudine's existing document behavior.             |
| Sixteen normalized provider-hook events, including session/tool activity                       | `claudine/lib/src/events/`, hook adapters, dispatch, generated provider metadata | Keep in Claudine. Bridging selected hooks to custom document events can be a later feature. |

The estimate assumes the first system is the intended subject. Migrating both would additionally involve provider schemas, hook registration, generated metadata, configuration, reporting, and compatibility with provider payloads. It would be a substantially different project.

Ownership also does not imply automatic execution everywhere Markdown is read. DMLS validation, schema detection, rendering, and ordinary passive inspection must never execute lifecycle actions. Even composition should run lifecycle events only through an explicit host orchestration boundary. A Claudine `success` means the provider attempt and completion checks succeeded; successful prompt composition alone cannot emit it.

## Measured footprint and limits of the evidence

The following counts are physical Rust lines, including comments and blank lines. Dedicated test files are paths beneath `tests/` or named `tests.rs`; the remaining files can still contain inline tests. These are inspection surfaces, **not predicted changed-line counts**.

| Source subtree                                 | Rust files | Total lines | Dedicated test files / lines | Other files / lines |
|------------------------------------------------|-----------:|------------:|-----------------------------:|--------------------:|
| `claudine/lib/src/composition/lifecycle/`      | 32         | 20,762      | 20 / 11,560                  | 12 / 9,202          |
| `claudine/lib/src/composition/looping/`        | 16         | 6,878       | 8 / 3,521                    | 8 / 3,357           |
| `claudine/cli/src/commands/wrap/harness_orch/` | 32         | 16,805      | 17 / 9,508                   | 15 / 7,297          |

The three non-overlapping subtrees total 80 files and 44,445 lines. Only the lifecycle subtree is the primary extraction candidate; neither moving all 44,445 lines nor rewriting them is warranted. Preparation, preflight, coordinator, sequence tasks, diagnostics, schemas, and CLI integration tests add touchpoints outside this inventory. A literal `LifecycleSignal` search found 63 files under Claudine's library and CLI source trees, including test modules; this indicates coupling, not 63 mandatory edits.

GitNexus was bound to this worktree and reported index commit `a0eedb5`, matching all 7,218 covered files. Its lifecycle concept query returned definitions but no execution processes. An upstream impact query for the exact `LifecycleSignal` enum returned **UNKNOWN**, with no resolved callers; context/impact queries could not resolve the executor's `execute_event` method. Those results are not evidence of low impact. Source inspection confirmed use in the looping engine, composition pipeline, terminal-event routing, and sequence task machinery. No complete graph-derived caller or process count is claimed.

Sniff's package inventory confirmed that this work spans at least the `darkmatter`, `claudine`, and `claudine-cli` crates, with `darkmatter-cli` and `dmls` involved if lifecycle execution and authoring support are exposed there. Provider generation and the rendezvous crates should not become implementation dependencies of Darkmatter.

## Categorize by the kind of change

The useful division is **move, decouple, generalize, integrate, and prove**. File size mostly predicts the first category; behavioral risk lies in the others.

| Workstream                        | Category             | Scale / risk            | Concrete work                                                                                                                                                                                                             |
|-----------------------------------|----------------------|-------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Grammar and typed actions         | Move + decouple      | Medium / medium         | Extract `actions.rs`, `action_shape.rs`, `signatures.rs`, `parse.rs`, `validate.rs`, and `source_map.rs`. Replace Claudine-owned errors and fixed event assumptions while retaining authored order and source provenance. |
| Event evaluation and live state   | Decouple             | Large / high            | Move reusable portions of `executor.rs` and context/state support. Preserve expression timing, `set` visibility, dispatch failures versus evaluation failures, and shell authorization.                                   |
| Transition and recovery protocol  | Decouple + integrate | Large / high            | Separate reusable decisions in `control.rs` and `runtime.rs` from stream summaries, session availability, proxy preparation, and provider relaunch. Preserve terminal and finalize accounting.                            |
| Caller-defined events             | Generalize           | Medium-to-large / high  | Replace fixed configuration slots and seven-key assumptions with registered event descriptors, deterministic lookup, validation metadata, and an explicit emission API.                                                   |
| Communication and host services   | Decouple             | Medium / medium-to-high | Remove `GlobalSettings`, `RuntimeMessagingSettings`, and concrete host configuration from the engine boundary. Retain terminal rendering, messaging routes, speech, and detached audio ownership in host adapters.        |
| Claudine adoption                 | Integrate            | Large / high            | Wire preparation, loops, `compose`, `inline-compose`, sequences, task setup/teardown, and recovery through the new engine without changing their semantics. Most orchestration remains in place.                          |
| Darkmatter CLI, schemas, and DMLS | Integrate            | Medium / medium         | Establish opt-in execution semantics for `md`, if desired; move shared schema definitions; provide passive lifecycle diagnostics and event descriptions to editor consumers.                                              |
| Compatibility and verification    | Prove                | Large / high            | Move engine tests, retain host integration tests, exercise custom events, preserve diagnostic projection, and update public docs and skills.                                                                              |

### What can move relatively directly

The action AST and parser already consume Darkmatter expressions. The executor already uses `EffectEngine`, `EffectiveState`, `LayeredLookup`, `InjectedGlobal`, and `SubtreeCompose`. These are strong extraction seams: no new expression language or parallel effect system is necessary.

The existing `ShellRunner` and `LifecycleEmitter` injection points also allow silent deterministic tests. They are useful starting points, but `LifecycleEmitter` currently accepts Claudine messaging settings and TTS configuration, so moving the trait unchanged does not establish a clean dependency boundary.

Even apparently mechanical files need review. Parsing validates sound names through Playa, errors belong to `CompositionError`, and removed legacy validation keys encode Claudine migration policy. The generic parser should not acquire every historical Claudine rule merely because those rules currently share a file.

### What needs architectural separation

`StackExecutionContext` carries both reusable state and host-specific settings. It references invocation `RuntimeState`, coordinator `ActionLocation`, communication settings, shell execution, and request context. `LifecycleErrorInfo` projects Claudine's diagnostic snapshots and error types. `runtime.rs`, despite its provider-neutral routing description, also imports `StreamExecutionSummary` and `RateLimitInfo`.

The target dependency direction must remain **Claudine → Darkmatter**. Darkmatter must not import Claudine to retain these conveniences; the existing dependency would become a cycle. Prefer a `darkmatter::lifecycle` module initially. A new leaf crate is justified only if an actual dependency or build-cost constraint requires it.

Darkmatter should accept neutral error snapshots, execution facts, and host services. Claudine should remain responsible for selecting its effective diagnostic and translating provider outcomes into those facts. A shared error envelope must preserve code, category, disposition, origin, severity, details, cause, and source location; converting errors to strings would break lifecycle conditions and editor diagnostics.

The engine can return typed control requests. Claudine should continue to execute provider retries, session resumes, proxy adoption, budget accounting, and final completion checks. Generic retry decisions do not require Darkmatter to understand provider session compatibility. `defer` is currently unsupported; implementing its scheduler is outside this migration.

## Custom events are a separate design obligation

The current shape is closed in several places: `LifecycleSignal::ALL`, `LIFECYCLE_EVENT_KEYS`, named fields on `LifecycleConfig` and its stacks, action-placement rules, source paths, interpolation exclusions, and schemas. A new enum variant alone would not make caller-defined events work.

A bounded first API should provide:

1. **Core identity and registration.** Reserve core names and register namespaced caller events before parsing. Reject duplicate registrations and unknown authored event names deterministically. Keep existing Claudine frontmatter spellings; decide the custom-event container syntax explicitly rather than reserving arbitrary top-level document properties.
2. **Passive descriptors.** Describe each event's payload shape, available globals, legal controls, error policy, and whether it may repeat. The same descriptors should drive parser checks, interpolation exclusions, schema/help output, and editor validation.
3. **Explicit emission.** A caller supplies an event ID, validated payload, and execution context. Registering a name does not discover when a provider or application event occurred; the caller owns that integration.
4. **Protected execution state.** A custom event inherits the run's actual preflight authorization. Registration cannot enable shells before `start`, replace a core terminal event, or bypass finalize accounting. Unsupported host controls must fail clearly before any attempted execution.
5. **Defined isolation.** Keep registration immutable during a run and payload/global namespaces separate from document state. Define repeated emissions and prohibit or bound recursive emission. Preserve existing sequence task isolation and ordered state mutation.

Use the six setup/terminal events as candidates for the universal profile; treat `loop` as an explicitly enabled repeat-run profile while preserving all seven events for Claudine. This is a proposed boundary, not a requirement to rename existing YAML. Arbitrary user-defined state machines, dynamically loaded plugins, and custom action languages should not be bundled into event extensibility.

## Behavioral contracts that dominate risk

| Contract                                      | Migration hazard                                                                                      | Required evidence                                                                                                                                                                                                      |
|-----------------------------------------------|-------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Initialization is shell-free                  | Moving the executor creates an alternate shell route or treats approval as sufficient permission.     | Reject shells even in dead initialization branches; block early catch/failure/finalize shells and bootstrap `$()`; prove `no_error` cannot suppress the prohibition.                                                   |
| One terminal slot and controlled finalization | Both library and host emit a terminal event, or terminal downgrade executes a stack twice.            | Compare ordered event/action traces, including success/blocked downgrade, evaluation failures, and repeated recovery.                                                                                                  |
| Early versus event-time evaluation            | Lifecycle strings compose prematurely, or state/context is recaptured inconsistently.                 | Test deferred spans, lazy current values, shared request context, and mutation visibility across events and attempts. Preserve current behavior during extraction; coordinate planned binding-time changes separately. |
| Distinct error channels                       | `no_error` starts hiding expression errors, or the displayed diagnostic differs from `err.*`.         | Assert dispatch, evaluation, and explicit-control failures separately, including source spans and cause projection.                                                                                                    |
| Proxy and retry identity                      | Target initialization, overlay provenance, shell approval, or refreshed launch configuration changes. | Keep canonical preparation and coordinator tests; verify proxy boundary reset, fresh retry reads, and resume compatibility refusal.                                                                                    |
| Passive authoring                             | Schema validation or DMLS loads a runtime adapter and performs effects.                               | Parse and validate representative lifecycle documents with effect services unavailable; assert zero shell, network, audio, and document writes.                                                                        |
| Shared stack machinery                        | Sequence setup/teardown diverges from document events after extraction.                               | Run sequence task and group tests through the same moved parser/executor, including private state and failure routing.                                                                                                 |
| Host process behavior                         | Speech re-exec helpers, signals, cancellation, or budgets are accidentally absorbed into the library. | Retain host adapters and targeted CLI integration tests; keep all audio tests silent and terminal/browser tests from taking focus.                                                                                     |

## Delivery sequence and de-risking measures

**Stage 1 — prove the boundary.** Inventory outward dependencies and freeze representative event traces before moving code. Build a small in-memory host that executes one core event and one registered custom event using fake emitters and a disabled shell runner. Include a typed failure and a `set` mutation. The exit criterion is a usable engine boundary with no Claudine dependency, not merely a compilable event enum.

**Stage 2 — move passive grammar and diagnostics.** Transfer AST, parsing, source mapping, and validation with their tests. Introduce a neutral lifecycle error boundary and temporary Claudine re-exports/adapters. Keep one semantic authority; do not maintain two evolving parsers. Add a passive shipped-document corpus test and schema parity checks.

**Stage 3 — move evaluation and reusable transitions.** Reuse Darkmatter's existing expression/effect machinery. Translate host settings and provider facts at the boundary. Compare old and new results only with fake effects: executing both engines against real shells or messaging would duplicate side effects. Include early failure, terminal downgrade, finalize error, proxy, retry, and sequence setup/teardown traces.

**Stage 4 — cut Claudine over.** Preserve existing YAML and provider behavior across direct composition, inline composition, loops, and sequences. Keep canonical document preparation, provider launch, completion verdicts, and budget enforcement in Claudine. Remove temporary duplicated implementation after parity; compatibility exports can remain where they reduce import churn without creating a second authority.

**Stage 5 — expose broader use.** Finalize registration policy, add a second non-agent caller, and decide whether `md compose` gets explicit lifecycle execution. Test unknown events, duplicate names, payload errors, unsupported controls, and unauthorized shells. Export passive descriptors for DMLS and documentation. A second caller proves reuse better than simply renaming Claudine types.

At each stage, use package-scoped `just test` and `just lint`; add `just test-l2` only where real terminal behavior needs proof, and headless browser coverage only for affected browser behavior. Use the canonical isolated CLI fixtures. Validate portable engine behavior and host adapter behavior on macOS, Linux, native Windows, and WSL2 according to the repository's existing coverage policy. This assessment does not propose additional speculative CI matrix cells.

The strongest de-risking measure is **separating ownership migration from behavior expansion**. Preserve existing lifecycle behavior first. Do not simultaneously redesign binding time, add the defer scheduler, unify provider hooks, or change retry semantics. Temporary re-exports are inexpensive; diagnosing several semantic changes in one regression is not.

## Effort and decision gates

For one engineer already familiar with both package areas, a reasonable initial planning allowance is **25–45 engineer-days**, including design, migration, tests, and documentation, but excluding review waiting time and unrelated CI failures. This is a judgment estimate, not a measured delivery forecast:

| Milestone                                                             | Planning allowance |
|-----------------------------------------------------------------------|-------------------:|
| Boundary spike and behavioral baseline                                | 3–5 days           |
| Passive grammar, diagnostics, and compatibility adapters              | 4–7 days           |
| Evaluator/state/transition extraction                                 | 6–10 days          |
| Claudine integration and regression fixes                             | 5–9 days           |
| Custom-event contract, second caller, authoring support, and closeout | 7–14 days          |

The range assumes existing syntax survives, most tests transfer, and custom events use a bounded registration model. A general workflow framework or migration of the provider-hook system would invalidate it. Activating lifecycle execution throughout Darkmatter's ordinary compose/transclusion paths would also require a separate assessment of nested-run scope and duplicate effects.

Re-estimate after Stage 1. Continue if a second host can use the engine without provider types, passive validation stays effect-free, and the shell/terminal-state invariants can be tested at one authority. Narrow the scope if those conditions require moving the entire coordinator or provider harness into Darkmatter.

The evidence supports proceeding with a bounded extraction spike. The cost is concentrated in host boundaries and behavioral proof; the existing expression infrastructure and substantial test corpus make a staged migration feasible.

## Source anchors

Paths below are relative to the repository root and identify the implementation inspected:

- `claudine/lib/src/composition/lifecycle/{mod,actions,parse,validate,source_map}.rs`: seven-event model, fixed slots, passive grammar, guards, and authored surfaces.
- `claudine/lib/src/composition/lifecycle/{executor,context,control,runtime}.rs`: execution services, diagnostic coupling, recovery decisions, and provider-summary coupling.
- `claudine/lib/src/composition/{runtime_state,completion}.rs`, `prepare/`, `coordinator/`, and `looping/`: state lifetime, completion verdict, canonical preparation, handoffs, and iteration.
- `claudine/lib/src/composition/sequence/task/mod.rs`: shared action parsing and execution outside document event stacks.
- `claudine/cli/src/commands/wrap/composition/pipeline.rs` and `harness_orch/loop_control/`: initialization, catch paths, terminal downgrade, proxy, retry/resume, and host wiring.
- `darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs`: existing shell-approval lifecycle; this is approval orchestration, not an existing equivalent of Claudine's seven-event engine.
- `darkmatter/docs/schemas/{claudine,claudine-types}.yaml` and `darkmatter/dmls/src/overlay/schema.rs`: existing passive authoring integration to preserve and extend.
- `claudine/docs/topics/lifecycle.md` and the Claudine/Darkmatter skill references: behavioral contracts and architecture context, checked against source where relevant.

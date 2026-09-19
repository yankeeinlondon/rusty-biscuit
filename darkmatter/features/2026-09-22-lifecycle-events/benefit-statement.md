---
prompt: |-
    <skill name="claudine">
    ::file ^.claude/skills/claudine/SKILL.md
    </skill>
    <skill>
    ::file ^.claude/skills/darkmatter/SKILL.md
    </skill>
    
    ## Task
    
    Currently we have a rich lifecycle event model in Claudine but the more I think about it the more I'm certain that Darkmatter is the better owner of this lifecycle system. Then Darkmatter would own core events but would expose an API for callers to add in their own events if the base events provided didn't suffice.

    What causes me pause is the amount of change involved but the risks are NOT your concern in this task!

    You're task is to investigate what the _benefits_ of moving the lifecycle model to Darkmatter would bring. Take your time and double check your work. This is an important task and important decisions will be made off the back of it. 
hash: ced20c4690f7de67-0cd9b42881c14460
last_updated: 2026-09-19
---
# Benefits of Darkmatter-owned lifecycle events

Darkmatter is a strong architectural home for the **document lifecycle language and its reusable execution engine**. It already owns the expressions, frontmatter, schema validation, composition context, and document effects on which that engine depends. Moving lifecycle ownership there would let those capabilities operate as one public contract, available to Claudine and other document-processing applications.

The largest benefit is broader access to a coherent document automation model: an author could express initialization, conditional actions, outcome handling, and finalization once, while each application supplies its own work, additional events, and execution capabilities. A caller-extensible event API is central to that benefit. Moving a fixed enum alone would deliver much less.

This statement evaluates benefits, not migration effort or risk. It distinguishes facts verified in the current checkout from proposed capabilities. Examples involving new callers or custom events describe opportunities, not existing support.

## What would become reusable

Claudine currently has two distinct event models:

| Model             | Current responsibility                                                                                                                                | Relevance to Darkmatter ownership                                              |
|-------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------|
| `LifecycleSignal` | Seven document events: `initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, and `loop`; conditional action stacks and control outcomes | The direct basis for a shared document lifecycle engine                        |
| `AgenticEvent`    | Sixteen normalized provider-hook events, including session, tool, model, permission, and subagent events                                              | A domain vocabulary that Claudine could expose through extensions where useful |

These models are separate in the source. Provider hooks are not simply additional variants of the composition lifecycle enum. The proposal can make their vocabulary available through an extension mechanism without making provider hook registration, native payload parsing, or session management responsibilities of Darkmatter. [Composition lifecycle model](../../../claudine/lib/src/composition/lifecycle/mod.rs), [provider event model](../../../claudine/lib/src/events/agentic_event.rs).

The useful ownership boundary is therefore larger than event names: lifecycle parsing, passive semantic validation, event-context rules, ordered stack evaluation, and common outcome routing belong alongside Darkmatter's document language. Applications provide domain events and perform domain operations. For example, Claudine still knows how to launch or resume a provider session; a publishing application knows when publication succeeded. The exact base-event set remains a design decision; the benefits below do not require declaring all seven existing events universally applicable.

## 1. Document automation becomes available without an agent wrapper

Today a Darkmatter caller can compose Markdown and call individual effects, but the lifecycle stack parser and executor live in the `claudine` crate. The dependency already points from Claudine to Darkmatter. A Darkmatter-owned engine would make conditional document workflows available through the dependency a document-processing application already uses. [Darkmatter manifest](../../lib/Cargo.toml), [Claudine manifest](../../../claudine/lib/Cargo.toml), [stack executor](../../../claudine/lib/src/composition/lifecycle/executor.rs).

That enables concrete new uses:

- A documentation builder could initialize supporting files, compose content, validate its output, and record the result.
- A report generator could run a conditional stack after successful generation and a different stack after a validation failure.
- A publishing application could supply a publication event and use ordinary lifecycle actions to update document metadata or append an audit record.

These callers would reuse the document language instead of implementing their own `when`, action ordering, state visibility, and failure handling. They would not need a provider session just to obtain those semantics. This is an API and dependency-boundary benefit; no build-time or binary-size savings have been measured.

Library ownership would also make explicit lifecycle execution in `md` feasible. It would not, by itself, cause existing `md compose` or rendering commands to execute handlers. The current effect engine is deliberately invoked by an orchestrator rather than automatically by the composition pipeline. [Effect engine](../../lib/src/effects/mod.rs).

## 2. Composition and lifecycle binding get one semantic owner

Claudine already uses Darkmatter's expression parser and `SubtreeCompose`. There is no separate lifecycle interpolator to eliminate. The remaining split is who decides **which values are deferred, which globals exist for an event, and when values are evaluated**.

Currently, Claudine maintains `LIFECYCLE_EVENT_KEYS` and uses Darkmatter's exclusion mechanism to preserve those subtrees during composition. Later, its executor resolves action values through Darkmatter using event-time state. Darkmatter's `InjectedGlobal` already accepts arbitrary eager or lazy globals. [Lifecycle keys](../../../claudine/lib/src/composition/lifecycle/mod.rs), [composition options](../../lib/src/markdown/compose/context/options.rs), [subtree composition](../../lib/src/markdown/compose/subtree.rs).

Putting lifecycle knowledge beside composition allows a registered event to carry its own deferral and context contract. A caller adding an event could inherit the same whole-value typing, interpolation behavior, and event-time scope as core events, without separately teaching composition which subtree to leave unresolved.

For authors, the benefit is consistent behavior across the body, lifecycle conditions, and action operands. For maintainers, expression changes and lifecycle binding rules can be developed and verified together. The gain is consolidation of the existing shared semantics, not a claim that moving modules creates a new expression language.

## 3. Custom events can inherit the full language

An extension API can make a new event more useful than a callback name. With declarative event metadata, a caller could provide an event identity, payload shape, available context, and applicable control/capability rules. Darkmatter could then apply the ordinary lifecycle parser, validation, expression evaluation, and stack execution to that event.

For example, a publisher might register an event conceptually named `publisher.after_publish`, supplying the published URL and artifact identity. An author could conditionally record those values using the same action syntax used for a core `success` event. The name and registration shape here are illustrative, not proposed final API syntax.

This gives application developers independent evolution: adding a domain event need not require changing Darkmatter's built-in enum or forking its parser. It gives authors continuity: learning one lifecycle language continues to pay off in another application.

The existing arbitrary-global support in `SubtreeCompose` is a concrete foundation, but it is not yet an event registry. Custom-event parsing, metadata, and dispatch would be new capabilities. To extend the benefit into DMLS, the editor must also receive a passive description of the caller's events; an opaque runtime callback alone cannot provide that knowledge. [Injected globals](../../lib/src/markdown/compose/subtree.rs).

## 4. Editor diagnostics can agree more closely with execution

DMLS already offers schema-driven completion, hover, navigation, and diagnostics for frontmatter. Darkmatter also ships Claudine schema definitions containing lifecycle events and stacks. The gain would be deeper semantic coverage, rather than the first availability of lifecycle editing support. [DMLS frontmatter provider](../../dmls/src/providers/frontmatter.rs), [Claudine baseline schema](../../docs/schemas/claudine.yaml).

There is a specific gap worth closing: the reusable schema currently describes `lifecycle-action` as a mapping of arbitrary string keys to `any`, while Claudine's parser and validators know the actual verbs, action forms, event restrictions, and expression surfaces. A shared passive lifecycle API could expose those rules directly to DMLS. [Lifecycle schema types](../../docs/schemas/claudine-types.yaml), [action parser](../../../claudine/lib/src/composition/lifecycle/parse.rs), [semantic validators](../../../claudine/lib/src/composition/lifecycle/validate.rs).

That would support earlier feedback for an invalid action shape, a control action in the wrong event, or an `err` reference where no error context exists. The author could receive the same semantic diagnosis while editing that execution would later produce. Registered custom events could participate as well.

This benefit requires wiring DMLS to the shared API. Passive analysis would inspect syntax and contracts without executing handlers, evaluating effectful expressions, or manufacturing runtime outcomes.

## 5. Effects and their lifecycle invocation can evolve together

Darkmatter owns `EffectEngine` and its public capability descriptors. Claudine owns the lifecycle dispatch that evaluates arguments, selects the effect method, projects failures, and mirrors relevant document mutations into live state. The executor's explicit dispatch includes frontmatter mutation, file/directory creation, line/JSONL append, and HTTP POST. [Effect catalog](../../lib/src/effects/catalog.rs), [effect dispatch](../../../claudine/lib/src/composition/lifecycle/executor.rs).

Shared ownership makes it possible to maintain an effect's description, operand validation, dispatch, and lifecycle behavior together. An effect improvement could then reach every lifecycle consumer through one implementation instead of requiring each application to reproduce the invocation adapter.

The immediate benefit is a clearer place to establish that consistency. Fully deriving dispatch and authoring metadata from one typed catalog would be an additional design improvement, not an automatic consequence of relocating files.

Application-specific presentation still fits this model. Claudine's messaging routes, speech configuration, and provider-aware status output can be supplied as capabilities. Other callers can supply appropriate emitters while reusing stack semantics. The existing `LifecycleEmitter` and shell-runner seams demonstrate that the executor already supports injected behavior. [Emitter interface](../../../claudine/lib/src/composition/lifecycle/mod.rs), [executor interfaces](../../../claudine/lib/src/composition/lifecycle/executor.rs).

## 6. State changes and completion can follow one document contract

The current engine already guarantees useful state visibility: a later action sees an earlier frontmatter mutation, and later events can observe mutations made in `start`. Dedicated tests exercise both behaviors. Moving the engine makes that existing behavior reusable by every adopting caller. [Mutation visibility tests](../../../claudine/lib/src/composition/lifecycle/executor/tests/mutation_visibility.rs).

There is a related opportunity around completion. Claudine's passive completion verdict delegates to Darkmatter schema validation at `SchemaPhase::Completion`, then routes the result to `success` or `failure`. It retains the launch-resolved schema rather than allowing a document to weaken its own contract mid-run. [Completion verdict](../../../claudine/lib/src/composition/completion.rs).

A shared lifecycle could accept such a verdict from any producer. A deterministic report generator and an agent-assisted writer could therefore share the meaning “the produced document satisfies its declared contract.” Each caller would still provide its own evidence: Claudine's inline body-change requirement, for example, is not automatically appropriate for every renderer or publisher.

This would make successful document outcomes more consistent across tools. It would not make action sequences transactional or undo external effects; those are separate capabilities.

## 7. Proven lifecycle ordering becomes reusable infrastructure

Claudine has already separated substantial transition logic from process execution. `LifecycleTransitionInput` supplies facts, `LifecycleTransitionDecision` returns the next decision, and `LifecycleCatchProtocol` coordinates catch handling, error precedence, and finalization. [Runtime routing](../../../claudine/lib/src/composition/lifecycle/runtime.rs).

Moving and generalizing the common portions would give other callers a tested basis for terminal-event selection, catch sequencing, and finalization instead of asking them to reinvent those rules. It also gives Claudine a smaller domain responsibility: report execution facts and carry out provider operations while the shared engine handles common lifecycle semantics.

The benefit is controlled reuse, not treating every operation as an agent session. `resume` needs a resumable operation; provider rate-limit handling needs provider facts. Extensions or host capabilities can retain those meanings while ordinary document operations reuse the common state machine. Nor does a finalization protocol promise recovery after an uncatchable process termination.

## 8. A shared engine can preserve execution policy across callers

Darkmatter already holds request-scoped composition authority and separate shell-preflight and effect-policy mechanisms. Claudine adds lifecycle rules such as shell-free initialization, a runtime shell boundary before `start`, and early resolution of shell commands so executed text matches approved text. [Composition options](../../lib/src/markdown/compose/context/options.rs), [shell preflight](../../lib/src/markdown/compose/preflight/lifecycle.rs), [lifecycle validation](../../../claudine/lib/src/composition/lifecycle/validate.rs), [guard and runtime boundary](../../../claudine/lib/src/composition/lifecycle/mod.rs).

A Darkmatter-owned lifecycle could carry those rules with the reusable engine and the same captured context. New callers would gain a standard way to distinguish parsing from execution, supply approved capabilities, and preserve document-relative resolution through event handling. Application-specific events could participate in those controls rather than introducing separate effect paths.

This is the positive value of a shared policy boundary. It does not imply that the existing filesystem, network, shell, and lifecycle policies are already one unified object, or that all applications must grant the same capabilities.

## 9. Diagnostics, documentation, and tests gain a common foundation

Lifecycle parsing currently has its own source map, and runtime failures carry property paths and typed error information. Darkmatter owns the underlying expression and schema errors, while DMLS has source-range projection. Shared ownership creates a natural place to expose lifecycle diagnostics that both execution and editors can consume. [Lifecycle source map](../../../claudine/lib/src/composition/lifecycle/source_map.rs), [error context](../../../claudine/lib/src/composition/lifecycle/context.rs).

The practical payoff is a shorter path from an error to the exact authored event, stack item, or operand, using the same diagnosis across tools. This requires retaining source provenance through the shared API; a move alone does not improve diagnostic ranges.

The existing executor tests use recording emitters and mock shell runners, and the transition logic has dedicated tests. Those tests demonstrate that much of the behavior can be verified without launching a provider. A shared implementation would let fixes to binding, mutation visibility, and catch ordering benefit every caller, with application tests focused on correct integration. [Executor test fixtures](../../../claudine/lib/src/composition/lifecycle/executor/tests/mod.rs), [runtime tests](../../../claudine/lib/src/composition/lifecycle/runtime/tests.rs).

A passive event/action descriptor API could additionally power reference documentation and capability reports, following Darkmatter's existing effect catalog pattern. This would keep custom events discoverable without requiring a live run.

## Which benefits follow from which decision

| Decision                                                                          | Benefit it unlocks                                                                           |
|-----------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| Move the reusable parser, validator, executor, and common routing into Darkmatter | Direct library access, semantic ownership alongside composition, and shared behavioral fixes |
| Add caller-defined event identities and context/capability contracts              | Domain-specific events that reuse the language without expanding a fixed core enum           |
| Expose passive descriptors and diagnostics to DMLS                                | Richer completion, event-aware validation, and consistent authoring feedback                 |
| Connect adopting applications to lifecycle execution and verdicts                 | Portable workflow behavior in actual document-generation and publishing tools                |
| Consolidate effect metadata and invocation further                                | Less independent maintenance between capability descriptions and executable action rules     |

The strongest case is the combination of the first two decisions: **Darkmatter becomes the common document automation language, and Claudine becomes a specialized execution host for it.** Editor parity and broader application adoption increase that value, but should be credited only when their integrations exist. Portability means shared syntax and semantics among hosts supporting the required events and capabilities; it does not mean every document can run unchanged under every host.

No performance, binary-size, or migration-cost claim is needed to support this conclusion. The code already shows that the lifecycle engine's core language dependencies point toward Darkmatter. Giving that engine a reusable home and an explicit extension model would make its existing sophistication useful beyond its first application.

## Evidence and verification

Investigated against the local checkout on 2026-09-19. GitNexus was bound to this worktree's registered repository, `rusty`, at commit `a0eedb5`; graph discovery was followed by direct inspection of the linked implementations, schemas, and tests. Some graph queries returned empty relationships or malformed search records, so they were not used as evidence of dependency absence. The index also reported a changed schema-support document; the canonical refresh was requested.

The behavioral claims above come from implementation and test-source review; no Rust tests or benchmarks were run for this documentation-only task. Existing tests are cited as evidence of the current contracts, not as newly passing results. The exact proposed event registry and additional integrations remain design work.

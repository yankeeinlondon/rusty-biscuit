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

    You're task is to investigate this change and take an adversarial position on why we should NOT move the lifecycle events.

    > Note: one obvious reason might be the scale of this change and you should mention this if you feel it is a risk but make sure to look for _other_ reasons that might block us from doing this migration.
hash: d632c4b05bae91f5-08a9579f41211544
last_updated: 2026-09-19
---
# Against moving lifecycle ownership to Darkmatter

The strongest reason to reject this migration is that **lifecycle ownership belongs with the component that owns execution and can establish whether an event actually happened**. Darkmatter supplies composition, evaluation, validation, and explicit effects. Claudine owns the provider attempt, preflight boundary, completion decision, recovery, and document handoff. Moving lifecycle ownership downward would either split that authority between two systems or turn Darkmatter into a workflow runtime.

That objection survives even if migration were free. The events themselves are document-centric (section 1 concedes this); the dispute is over who governs the run in which they fire.

## Scope and evidence

The subject of the proposed migration is the **document lifecycle**: the seven events `initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, and `loop`, together with the lifecycle hooks (stacks) a document's frontmatter attaches to them and the control semantics those stacks carry. Source: [document lifecycle types](../../../claudine/lib/src/composition/lifecycle/mod.rs).

The investigation used the current worktree's GitNexus repository alias `rusty`; its index matched commit `a0eedb5` and all covered files on September 19, 2026. Graph queries were followed by source inspection. An upstream impact query for `LifecycleEmitter` reported **CRITICAL** risk; the broad query was partial, and a depth-one rerun identified `DefaultLifecycleEmitter` as the direct implementation. Process enrichment included both Claudine sequence/CLI flows and unrelated-looking paths, so those results are a warning to investigate, not a defensible count of affected callers. The arguments below rest on explicit source contracts rather than graph totals. No implementation migration was attempted.

## 1. One stage of the document's transformation happens outside Darkmatter

All seven events describe stages in the transformation of one document, and that is the strongest point in the migration's favor. The contract's own pipeline reads `initialize → launch validation → start → provider → inline closure → verdict → success | failure → finalize`, and every step but one is a document operation. The completion verdict that chooses between `success` and `failure` asks only document questions: did the body change meaningfully, and does the document satisfy the schema resolved at launch? Both are answered with Darkmatter primitives. See the [lifecycle contract](../../../claudine/docs/topics/lifecycle.md) and [completion implementation](../../../claudine/lib/src/composition/completion.rs).

The exception is the step in the middle. The actor that transforms the document is an external process that Darkmatter neither launches nor observes, and three events turn on facts about it: `blocked` means it was never started (a preflight denial, a refused write grant), `start` means it is about to be, and `failure` covers its error exit, timeout, interruption, or abort as well as a failed verdict. `err` must describe whichever of those occurred.

This is a design requirement rather than a blocker. A Darkmatter-owned lifecycle needs a host-reported outcome for the transformation step, with enough structure to populate `err`, and it must not fire `success` from a completed composition alone; the verdict has to run after the host reports the transformer finished. The objections that follow concern what the host must still decide around that step, not the meaning of the events.

## 2. Lifecycle stacks are control flow, not merely notifications

The stack executor returns requests to skip a document, retry an attempt, resume a provider session, proxy another document, or stop. The runtime makes decisions using launch state, session availability, attempt budgets, terminal-slot state, proxy history, and finalize state. It distinguishes evaluation failures, action failures, and explicit `error` control; those channels have different effects on the run. Sources: [executor](../../../claudine/lib/src/composition/lifecycle/executor.rs), [control dispatch](../../../claudine/lib/src/composition/lifecycle/control.rs), and [runtime transitions](../../../claudine/lib/src/composition/lifecycle/runtime.rs).

There is already a useful separation: pure transition decisions live in the Claudine library, while the CLI performs process and rendering work. Provider-neutral does not mean workflow-neutral. A resumable provider session remains an agent-execution concept even when the decision code carries only facts and no process handles.

A migration would have to choose between leaving substantive ownership in Claudine behind callbacks or importing its execution concepts into Darkmatter. A generic callback interface can avoid a Rust dependency cycle; it cannot eliminate the semantic dependency. It may instead create a large host protocol whose implementers must reproduce Claudine's invariants.

`defer` is especially unsuitable as evidence of a ready-made universal runtime: the current transition model explicitly represents deferred execution as unsupported. Moving its declaration does not supply durable scheduling semantics.

> ASSESSMENT: overstated. The coupling to agent execution is much thinner than this section implies.
>
> The section's own second paragraph gives the answer: the transition logic is already pure and carries only facts. The source bears that out. `control.rs` imports nothing from Claudine outside the lifecycle module, and `runtime.rs` imports a single outside type, the stream summary that reports a run's outcome. That is a state machine which can move as it stands. `skip`, `stop`, `error`, and `proxy` are document-level flow with no agent concept in them, and `retry` means "run the transformation step again". Only `resume` needs a host fact (is a session available?), which is one capability query. The warning that hosts "must reproduce Claudine's invariants" runs backwards: the invariants live in the state machine, so moving it puts them in one place, and a host implements only a narrow contract (launch the step, report its outcome, say whether it can resume). What remains true is that this contract has to be designed, and that session compatibility and launch-bundle recomputation stay host-side behind it. The `defer` paragraph is not an argument either way, since `defer` is unimplemented under both owners.

## 3. Safety depends on execution phase, not event registration

Initialization is shell-free, including dead branches and bootstrap frontmatter shell expansion. Early blocked/failure/finalize paths must not provide an escape. Lifecycle shells become available only at `start`, after preflight; proxy adoption resets that boundary. Approval flags and `no_error` cannot override the prohibition. These are binding rules in the [lifecycle contract](../../../claudine/docs/topics/lifecycle.md).

A custom event named `after_prepare` raises questions that an event identifier cannot answer: Has its shell text been audited? Is it firing before or after launch? Can it reach a catch handler? Can it proxy a new document? Which authority permits its effects?

If an extension can simply declare itself safe, it can bypass the core phase restrictions. If Darkmatter independently verifies those facts, it needs the host's execution state. If the host remains responsible, it remains the safety authority. **A design that cannot demonstrate one authoritative phase gate should block migration**, regardless of how attractive the registry API looks.

> ASSESSMENT: overstated for the core events; a fair rule for registered ones.
>
> The closing trilemma assumes Darkmatter would have to obtain phase state from the host. If Darkmatter owns the lifecycle, the phase *is* its state, and it becomes the "one authoritative phase gate" that this section demands. The gate is already implemented inside the lifecycle module, as a `DisabledShellRunner` that is swapped for the real runner once preflight has passed, so it moves with the code. The host contributes one fact (preflight passed) and one policy (its command approval list), which is an adapter rather than shared authority. The residue concerns registered events only: an extension must not choose its own runner. Darkmatter can enforce that by assigning the runner from the position an event is registered at, so that anything before `start` is shell-free by construction. That is a rule for the extension API, not a reason to leave the core lifecycle where it is.

## 4. Reuse already exists without transferring ownership

The current executor uses Darkmatter's `EffectEngine`, expression evaluation, `SubtreeCompose`, `LayeredLookup`, and injected globals. Lifecycle context explicitly delegates event-time interpolation to Darkmatter rather than maintaining a separate expression lookup. Sources: [executor imports and contract](../../../claudine/lib/src/composition/lifecycle/executor.rs) and [context implementation](../../../claudine/lib/src/composition/lifecycle/context.rs).

Darkmatter's [effect engine](../../lib/src/effects/mod.rs) is deliberately callable by an external orchestrator and is not wired into the compose pipeline. This is a valuable separation between possessing an operation and deciding when to perform it.

This argument does not assume Darkmatter is pure: its composition pipeline already permits approved shell and remote work. The narrower boundary is that ordinary composition does not automatically execute lifecycle effect stacks, and passive validation/editor features must remain passive. Moving types or a parser alone need not break that boundary; automatically executing recognized lifecycle frontmatter would.

Before changing ownership, identify a concrete capability that cannot be delivered through the existing shared evaluator/effect APIs. Eliminating a wrapper around those APIs is not sufficient justification for relocating the workflow contract.

> ASSESSMENT: overstated. The pattern this section praises is the template for the migration, and the capability it asks for has already been found.
>
> `EffectEngine` lives in Darkmatter, is callable by an external orchestrator, and is not wired into the compose pipeline. A Darkmatter-owned lifecycle can be offered on exactly those terms: a library API that a host drives, with nothing executed by `md` or by ordinary composition. Nothing in the proposal requires "automatically executing recognized lifecycle frontmatter", so the third paragraph answers a design nobody has put forward. The challenge to "identify a concrete capability" also has an answer in the current tree. Lifecycle knowledge is already duplicated across the boundary: DMLS keeps its own copy of the event paths and late-binding roots, and Darkmatter's subtree-composition and exclusion APIs exist chiefly to serve it. The existing arrangement therefore does not deliver a single definition of the lifecycle that the runtime, the editor, and any future Darkmatter consumer all share; the migration does.

## 5. Splitting the events across an extension API divides one state machine

Under the proposal Darkmatter owns a core subset of the seven events, and Claudine registers the remainder through a programmatic extension API. The difficulty is that the seven are not independent notifications that can be apportioned between two owners. They are the states of one machine: exactly one of `success`, `blocked`, and `failure` occupies an iteration's terminal slot; a `success` or `blocked` stack can redesignate that slot to `failure`; `finalize` follows whichever fired, exactly once; `loop` gates only after `finalize`. See the [lifecycle contract](../../../claudine/docs/topics/lifecycle.md) and [terminal event orchestration](../../../claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs).

The events Claudine would most plausibly register are those at the end of the lifecycle, after the document's transformation has completed, and `loop` is the clearest case. Even there the invariants cross the line. A registered `loop` event must run after the core `finalize`, evaluate its gate, apply its per-iteration mutations, and then send the core machine back to `start` for another pass, skipping `initialize`, schema validation, and shell preflight, with the core `success`, `failure`, and `finalize` firing again on each iteration. An API that expressive does not add an event to the end of a sequence; it lets a caller re-enter Darkmatter's state machine at a state of its choosing and bypass the core's own gates, and Darkmatter must keep its invariants true against registrations it has never seen. An API that is less expressive can host only notification-style events, so `loop` and anything like it must be core, and the iteration model moves into Darkmatter with it.

A programmatic registration also exists only inside the host process that made it. DMLS is a separate process that never links Claudine. Today it recognizes all seven events and the late-binding globals legal beneath each, because the set is fixed and the editor can mirror it ([DMLS frontmatter diagnostics](../../dmls/src/diagnostics/frontmatter.rs)). After the split it would know the core events and nothing else.

Suppose `loop` is an event Claudine registers, and `review.md` asks for three review passes:

```yaml
iteration: 1
max_iterations: 3
loop:
  while: "iteration <= max_iterations"
  actions:
    - increment(iteration)
  stack:
    - action: { info: "finished pass {{_loop_count}}" }
```

The same file now behaves three different ways depending on which process reads it:

- **`claudine compose review.md`** works. Claudine registered `loop` at startup, so the subtree is deferred to event time, the gate runs after each `finalize`, and the document is reviewed three times.
- **The editor** has no registration to consult. If DMLS treats `loop` as ordinary frontmatter, it reports `{{_loop_count}}` as an undefined variable, a false error on a correct document. If it stays quiet about keys it does not recognize, it also stays quiet about `{{_loop_cout}}` and about a malformed `while` expression, mistakes the author will now discover only at the end of the first pass. Today DMLS can check these because `loop` is in its fixed list.

> CORRECTION:
> 
> The editor -- via DMLS -- is immediately **more** capable in enforcing and communicating lifecycle events due to Darkmatter's ownership. We also have a method for DMLS to support lifecycle events regardless of owner that will roll out in the next day or two and so any "additional" events that Claudine were to add would also get equally good DMLS support.
    

- **`md`, or a second host built on the Darkmatter lifecycle** that never registered `loop`, has three choices, all poor. It can compose the subtree as ordinary frontmatter, in which case `{{_loop_count}}` is evaluated at compose time, where it does not exist. It can ignore the key, in which case the document runs once, fires `success`, and exits cleanly, and nothing tells the author that three passes became one. Or it can reject the unknown event, in which case a document that is valid under Claudine is an error everywhere else.

> CORRECTION:
> 
> this is a non-issue. things only improve in this migration because all core lifecycle events are immediately covered in DMLS or any library user of Darkmatter. Events which a caller like Claudine register will have the ability to provide schema support to DMLS through a spec that should land in a day or two.

Closing the editor gap requires a second channel, such as a descriptor that Claudine exports and DMLS loads, and that descriptor must be kept in step with the programmatic registration it duplicates. Closing the host gap requires every host to register every event any document might use.

Keeping all seven events in one owner avoids both problems. The state machine has a single author, and the editor mirrors one fixed list.

## 6. Composition scope is not run scope

Claudine's lifecycle can outlive one composition call. Retry and resume reread and audit a document without initializing it again; proxy adoption changes the active document and its lifecycle ownership. Loop iterations and sequence steps introduce further boundaries. The [lifecycle contract](../../../claudine/docs/topics/lifecycle.md) records these differences, including the rule that dry runs fire no lifecycle events.

Darkmatter also composes nested/transcluded documents. If lifecycle firing is attached to composition generally, does each included document initialize and finalize? Does rereading a retry target repeat initialization? If execution is restricted to the root, which root applies after a proxy? Caching composed content cannot justify replaying or skipping arbitrary external effects.

These are prospective failure modes, not claims that today's code does them. They show why a request-scoped composition context is insufficient as the identity of a longer-running workflow. A separate run/attempt/document identity and explicit dispatch boundary would still be needed.

> ASSESSMENT: largely a strawman, as the section itself half-admits ("prospective failure modes, not claims that today's code does them").
>
> Every question in the second paragraph follows from the premise that lifecycle firing would be "attached to composition generally". The proposal does not say that, and the obvious design does not do it: the lifecycle would be a run-scoped object in Darkmatter that a host creates and drives, separate from `compose()`, exactly as Claudine's lifecycle is separate from Darkmatter's composer today. Transcluded documents fire no lifecycle events now and would fire none afterwards. Retry, resume, and proxy re-entry rules are state-machine behavior that moves with the state machine. The genuine point is the last sentence: a run/attempt/document identity is needed. It already exists in Claudine (the document coordinator and the lifecycle runtime), so the work is to move or mirror it, not to invent it. Sequences and run budgets sit above a single document's lifecycle and would remain Claudine's.

## 7. Most of the error vocabulary describes work Darkmatter does not do

`err` is what a lifecycle recovery condition reads, as in `when: "err.category == 'cap'"`. Its structure is generic: a code, a category, a disposition, a severity, catalog-shaped detail, and a one-level cause. Darkmatter could own that structure as a typed model which hosts populate and extend, and doing so would not be difficult.

The vocabulary carried in that structure is another matter. Of Claudine's twelve error categories, only `composition` is Darkmatter's own. Seven describe an agent run (`auth`, `cap`, `timeout`, `provider`, `vcs`, `config`, `runaway`) and the remaining four are shared plumbing. A Darkmatter-owned model must therefore leave `category` and `code` open to values it does not define. That gives up the closed `Category` enum that Claudine matches exhaustively today, and it means a recovery condition misspelled as `err.category == 'cpa'` can be caught only if every host also declares its codes to Darkmatter. See [diagnostic facets](../../../claudine/lib/src/diagnostics/facets.rs).

One invariant must also survive the move. Rendering, `err.*`, and machine output all project the same selected diagnostic, so automated recovery and the report a person reads never describe different failures. The selection walk that picks that diagnostic is host logic. The host has to convert into Darkmatter's error type at exactly that point, and no error may reach `err` by a path that bypasses the walk, including errors Darkmatter raises itself. See [lifecycle error context](../../../claudine/lib/src/composition/lifecycle/context.rs).

Lifecycle communication likewise depends on host settings and adapters for terminal output, messaging, speech, and sound. The current library depends on Darkmatter, and its lifecycle module imports Claudine's user settings (`GlobalSettings`, `TtsSettings`) and messaging types. Sources: [Claudine manifest](../../../claudine/lib/Cargo.toml) and [lifecycle module](../../../claudine/lib/src/composition/lifecycle/mod.rs).

A literal move retaining those imports would create a dependency cycle. Proper extraction can avoid it, but requires an explicit adapter boundary. Darkmatter should not acquire default host behavior merely because an event is recognized; unsupported effects need explicit outcomes, and library/editor consumers should not accidentally inherit workflow initialization or notification behavior.

> ASSESSMENT: the error-model concern above is minor and should not weigh on the decision.
>
> Both points have straightforward answers, and neither describes a loss relative to today. Darkmatter can define the error structure with its own core categories plus a host-supplied category type (a generic parameter or a `Host(H)` variant), so Claudine keeps a closed enum and exhaustive matching on its side of the boundary; the typing would be at least as strong as it is now. The misspelled `err.category == 'cpa'` is not caught today either, since nothing in Claudine or DMLS validates that literal against the category list, so a Darkmatter-owned model with host-declared codes could only improve on it. The single-projection invariant is preserved by converting at the one point where the host already selects its diagnostic, and a Darkmatter-owned lifecycle that receives every failure through the host-reported outcome has only that one path into `err`.

## 8. Migration scale is a multiplier, not the principal objection

The difficult compatibility surface is behavior: event order, terminal-slot redesignation, finalize eligibility, error precedence, shell audit, completion verdicts, and re-entry. For example, a `success` or `blocked` stack can explicitly downgrade the run to failure after its top-level communication has already occurred. A generic “one terminal callback” abstraction could erase this intentional behavior. See [terminal event orchestration](../../../claudine/cli/src/commands/wrap/harness_orch/loop_control/lifecycle_events.rs).

Extracting code while also introducing open-ended events changes two dimensions at once. Passing parser tests would not establish runtime equivalence. Verification would need normal invocation paths for blocked preflight, completion failure, catch evaluation failure, retry, resume, proxy, loop, sequence, dry run, and platform-specific effect adapters. The cost matters, but even perfect migration tests would not fix an incorrectly assigned authority boundary.

## Recommendation and conditions for reconsideration

Keep ownership of the existing document lifecycle in Claudine. Continue sharing Darkmatter's expression, schema, context, and effect primitives. If another caller needs reuse, first establish its concrete event semantics and extract only the common mechanism demonstrated by both callers. A small passive descriptor/parser surface or a separate workflow crate may prove appropriate; neither requires Darkmatter to own the semantics of a launched agent attempt.
Reconsider broader ownership only when the proposal supplies all of the following:

1. A defined host-reported outcome for the external transformation step, structured enough to drive `blocked`, `failure`, and `err`, with the completion verdict running only after it.
2. One execution authority for phase permissions, recovery, terminal outcomes, and finalization.
3. An extension API that preserves terminal-slot and ordering invariants across core and registered events, and a channel through which DMLS and other hosts learn of registered events.
4. Explicit scope rules for transclusion, retries, proxies, loops, sequences, caching, and dry runs.
5. A second concrete consumer demonstrating shared semantics, plus a migration plan that preserves current behavior before generalizing it.

The strongest pro-migration point is real: lifecycle authors already use Darkmatter's language and effects. The existing architecture already captures much of that benefit. The case against migration is that ownership should follow the authority to govern a run, and the inspected code places that authority in Claudine.

> ASSESSMENT of the overall case: weaker than the recommendation above suggests.
>
> Read against the source, most of the objections reduce to design requirements rather than reasons to keep ownership where it is. Section 1 concedes that the events are document-centric. Sections 2, 3, 4, 6, and 7 carry assessments showing each to be overstated or answerable. Two editor and portability claims in section 5 are corrected in place. The thesis that "ownership should follow the authority to govern a run" also overreaches: what Claudine uniquely governs is a single step, launching the transformer and reporting what happened to it, while the machine around that step is pure logic over document state. Condition 5 asks for a second consumer, but one already exists, since DMLS consumes the lifecycle vocabulary today through a hand-maintained copy.
>
> What survives is real but modest. First, the host contract must be designed deliberately: the reported outcome of the transformation step, the resume capability, the preflight fact, the approval policy, and the effect adapters. Second, section 5's state-machine point stands: either `loop` is a core event or the extension API must support re-entry into the core machine. Third, section 8 stands as written: the compatibility surface is behavioral, and the migration should move the existing lifecycle unchanged before opening it to registered events. These are conditions on how to migrate. None of them is an argument against migrating.

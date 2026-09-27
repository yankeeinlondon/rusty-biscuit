---
area: claudine
status: draft-spec
created: 2026-09-27
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6-astra
reviewed_on: 2026-09-27
review_iterations: 0
origin: ./suggestions.md
depends-on:
    - 2026-09-21-lifecycle-ergonomics
related:
    - 2026-07-11-sequence-plus
    - 2026-09-20-lifecycle-handoff-gaps
---

> **Sequencing.** This feature is implemented after `2026-09-21-lifecycle-ergonomics`,
> which removes the `stack:`/`action:` lifecycle grammar every example below is
> written in and defines the action enumeration that `break` and `prep` join.
> The last task of that feature rewrites this spec's examples into the new
> grammar; until that has happened, read the examples for their semantics, not
> their spelling, and do not start implementation.

# Sequence improvements

## Problem

Claudine runs three executable primitives: a Markdown document, a group of
tasks, and a sequence of steps. Only the document can loop, only the document
can decide not to run, and the built-in state a sequence injects shares a
namespace with the author's own frontmatter. Writing a sequence that
alternates two shipped prompts until a condition holds therefore required:

- an unrolled 21-step list to express a five-cycle loop, with the cap written
  as prose because nothing could read it;
- `skip` gates added to three shipped prompts that are also run standalone,
  guarded by an `in_loop` flag whose only purpose is to say "you are inside a
  sequence";
- renaming a prompt's own `previous` key, because the sequence overlay
  shadowed it silently and the failure surfaced five steps later as
  `basename()` receiving an object;
- a `shell: git add .` step before every commit, which `--dry-run` executes for
  real;
- and it still could not run a multi-phase plan, because a looping document
  placed in a sequence runs exactly one iteration and reports success.

That last point is a defect, reproduced in **Evidence** below with a small
document and a one-step sequence.

## Decisions

Made by the owner on 2026-09-27. Review clarifications below preserve these
goals; unresolved changes to existing contracts are called out in **Open questions**.

1. **Every loop behaves identically**, or as close to identically as the
   primitives allow, whichever primitive carries the `loop:` block.
2. **The loop condition is checked after the primitive has run**, as the
   document loop does today. `when:` decides whether a primitive runs at all;
   `loop:` decides whether it runs again.
3. **Leaving one iteration stays `skip`. A new `break` directive leaves the
   loop.** Today `skip` is accepted only in `initialize` and opts the whole
   document out, so honoring the first half needs a placement decision; see
   **Open questions — Skipping an iteration**.
4. **A new `prep` directive calls another primitive and returns**, where
   `proxy` hands off and never returns.
5. **Built-in state moves under `state`**, which is already reserved, so that
   userland names cannot collide with it. The step's own `state` object is
   unchanged; `loop` and `seq` become the two keys an author may not use in it.
6. **`defer` suspends the whole run**, not one step, and lands only when the
   `rendezvous` daemon can schedule a resumption. Its contract is fixed here;
   its implementation is out of scope.

The user-facing description of the directives is already published, marked
planned, in [Flow Control](../../docs/topics/flow-control/flow-control.md) and
[Looping](../../docs/topics/flow-control/looping.md). See **Specs and docs**
below for how this document and those pages relate.

## Specs and docs

This repository keeps two kinds of writing about behavior, with different
jobs:

- **A spec is a snapshot in time.** It records what was decided, why, and what
  the implementation must achieve, as of the date it was written. Once
  implemented and closed it moves to `_completed` and is not maintained.
  Nothing should treat a spec as the current description of how Claudine
  behaves.
- **The `docs/` tree is the current record.** A topic page describes how the
  feature behaves now, and where behavior is not yet built the page says so
  with a **planned** marker rather than by pointing elsewhere. When behavior
  changes, the page changes in the same commit.

Three rules follow, and this feature is held to them:

1. **A spec may link to docs; a doc never links to a spec or fix.** A doc that
   says "tracked in `2026-…-something`" has delegated its content to a
   snapshot that will move and go stale. The doc states the behavior, or the
   defect, in its own words. Prose references by date-name are covered by the
   same rule.
2. **Planned behavior is documented before it is built.** The flow-control and
   looping pages already describe `break`, `prep`, `defer`, `when:`, uniform
   loops, and the `state.loop` / `state.seq` names, each marked planned. Each
   piece of this feature removes the corresponding marker in the same change
   that lands the code. A marker that outlives its implementation is a defect
   of the change that landed it.
3. **When the implementation departs from this spec, the docs are corrected,
   not the spec.** The departure and its reason go in the implementation log;
   the doc page is updated to describe what was built; this document keeps
   saying what was decided on 2026-09-27.

The inline review edits this specification only. Reconcile the affected topic
pages with the approved choices before implementation starts, keeping new
behavior marked planned. Those pages remain the current behavior reference.

## Executable primitives

The term **executable primitive** means one of:

| Primitive | Defined by | Runs |
|---|---|---|
| document | a Markdown file with frontmatter | its body through a provider, or `inline-compose` |
| group | a `group:` object, inline or in a `kind: group` file | its `tasks`, serial or parallel |
| sequence | a document whose frontmatter has a `sequence:` key | its steps in order |

A sequence step that names a `prompt:`, `group:`, or `task:` runs that
primitive; a step with no executable runs the sequence document's own body.
Everything below is stated for a primitive and applies to all three unless a
row says otherwise.

## Uniform `loop:`

### Placement

`loop:` is accepted on all three primitives with one grammar:

```yaml
loop:
    until: <expr>        # or while: <expr>; exactly one
    action: <mutation>   # optional, applied only when the loop continues
    max: <number>        # optional, positive integer; default cap is 100
```

- A document carries it at its frontmatter root, as today.
- A group carries it on the group object, beside `execution` and `tasks`.
- A sequence document carries it at its frontmatter root. Presence of
  `sequence:` decides that the root block belongs to the sequence and not to
  the document's body. A file with both `sequence:` and a body is an inline
  sequence today; its root `loop:` is the sequence's.

A document's loop keeps working when that document runs as a sequence task or
a group task. This is the defect fix. `shell:` and `side_effect:` tasks repeat
by belonging to a looped group; this does not add `loop:` to every task object.

The abbreviated example does not remove existing grammar: preserve `actions`
as an alias of `action`, list-valued actions, `fail_fast`, `on_rate_limit`, and
loop lifecycle concerns. `when` inside a loop lifecycle stack is an action
condition, not a new `loop.when` configuration key. Reject unknown keys.
Resolve templated caps once before entering the loop; reject zero, negative,
fractional, unresolved, and overflowing values. Preserve the existing runtime
option/environment precedence over authored configuration and the default cap
of 100. Apply overrides to each loop independently; they are not a total
provider-launch budget. Retries within an iteration do not advance its count.

### Semantics

Post-iteration checking is preserved. Exhaustion reporting and loops on
containers are new behavior; the rules below distinguish those changes.

- **Timing.** Enter the loop, run the iteration, complete its cleanup and
  loop lifecycle stack, then evaluate `until` or `while` against the current
  runtime values and fresh file reads. A loop runs at least once unless its
  task's `when` is false or its initialization opts out. Do not freeze the
  condition's input before the iteration.
- **`action`.** Apply mutations only if another iteration will actually run:
  after a continuing condition, before the next preparation, and never after
  the cap, a break, cancellation, or failure that stops the run. Document
  actions update in-memory loop frontmatter, not the source file. Container
  actions update the invocation's runtime `set` layer. Preserve atomic action
  staging: an invalid action commits none of that action list's writes.
- **Lifecycle.** A document invocation initializes once and fires `start`,
  its terminal event, and `finalize` per iteration under the existing lifecycle
  guard. When an outer group repeats a child document, that is a new child
  invocation, with a fresh initialization and inner-loop count. A child skipped
  on one outer pass may run on the next. Groups retain task `setup`/`teardown`;
  this feature does not implicitly add document events to group objects.
  A looped sequence initializes once; its root start/terminal/finalize events
  surround each pass through the steps, without launching its body as an extra
  task. This is new container lifecycle behavior and must use the same guard
  as document loops. Preserve cleanup and error-routing guarantees.
- **Outputs.** Keep the existing accumulator shape after moving it to
  `state.seq.outputs`. Serial tasks append individual strings across iterations;
  there is no extra group or iteration wrapper entry. A completed parallel
  group retains its declaration-ordered array entry. A looping prompt task
  publishes its final successful output once to its parent task accumulator;
  its internal iterations use their own local output history. Record iteration
  history in execution summaries without duplicating parent output entries.
  Skipped, failed, and interrupted serial tasks append nothing. Preserve the
  existing document-event visibility and task teardown publication boundaries.
- **Live re-read.** Re-read prompt content and frontmatter at the existing
  task preparation boundaries. Freeze the normalized step/task graph, loop
  configuration, and approved shell bytes for the invocation. Disk edits must
  not introduce an unapproved executable on a later iteration. A changed
  execution target or shell plan requires validation and approval before use;
  in unattended mode, fail clearly if that approval is unavailable.
- **Outcomes.** Normal loop completion has three outcomes, distinguished in the
  summary. Failures, handoffs, cancellation, and budget stops remain separate
  execution outcomes and must not be mislabeled as condition completion:

  | Outcome | Meaning | Exit |
  |---|---|---|
  | condition met | `until` became true or `while` became false | success |
  | `max` reached | the final permitted iteration still requires another pass | `1`, with a distinct typed exhaustion error and summary outcome |
  | broken | a `break` fired inside the loop | success unless the `break` says otherwise; see below |

  Test the condition before classifying exhaustion: meeting it on the last
  permitted pass succeeds. The typed exhaustion result is new; retain the
  sequence's existing exit `1` for failures instead of allocating an
  undocumented process code. Remove counter workarounds only after coverage
  proves equivalent failure reporting.
- **`fail_fast`.** A failing task or step inside a looped group or sequence
  follows existing scheduling boundaries: a serial group stops at its first
  failed task regardless of sequence `fail_fast`; with `fail_fast: false` the
  enclosing sequence can continue. A failed loop iteration may reach its next
  condition check only when its effective loop policy allows that. Preserve
  failure in the final aggregate even if a later condition becomes true;
  neither condition completion nor a zero-code break erases an earlier failure.
- **Ambient values.** `state.loop.count`, `state.loop.is_first`, and
  `state.loop.is_last` replace `_loop_count`, `_loop_is_first`, and
  `_loop_is_last` (see **State**).
  They are readable from the primitive's own lifecycle events and from every
  task or step inside it. Conditions inside the loop lifecycle stack and `until`/`while`
  can read them too; ambient values must not be mistaken for authored control
  variables during seed extraction.

### Shared scheduling without changing sequence state

Root sequence loops and serial group loops share the repetition mechanism,
not an exact normalization into the same group object. Sequence steps retain
their own `state`, schema, neighbor relationships, and failure policy. Groups
retain group-local variables and their first-failure scheduling rule. Wrapping
every step in an ordinary group would change those behaviors.

For two successful serial tasks producing `A` and `B`, two passes produce
`["A", "B", "A", "B"]` in either container's owning accumulator. Iteration
numbers belong in summaries and `state.loop`, not extra output nesting.

### Nesting

Loops nest: a looped document inside a looped group inside a looped sequence.
`break` and the ambient `state.loop.*` values refer to the innermost enclosing loop.
Loop nesting does not consume handoff hops. Referenced document cycles still
require detection during preflight; adding loops must not permit recursive
file references. See the parallel execution restriction under **Open questions**.

## `when:` on a primitive

A `when:` expression is accepted on any sequence step and on any group task,
beside the executable field. It is evaluated at the primitive's turn, against
the live state at that moment, with the same expression surface as a lifecycle
`when`.

```yaml
sequence:
    - name: implement
      prompt: "@prompts/_implement/implement-plan.md"
      when: "frontmatter(spec, 'implemented') != true"
      params:
          spec: "{{ spec }}"
```

Here `spec` is a caller-supplied file parameter on the sequence document.

- A false `when:` records the primitive as **skipped**: nothing is composed,
  no provider launches, no lifecycle event fires, its loop runs zero times,
  and nothing is appended to `state.seq.outputs`.
- Evaluate `when` before task setup. False skips setup and teardown as well
  as the executable. Reevaluate it on every outer iteration.
- Static preflight still validates all declared branches, including false
  ones; `when` cannot hide an invalid command or bypass shell approval.
- An expression error fails the task with its source location; it is not false.
  Parallel tasks evaluate against their group's shared start snapshot.
- A skipped primitive is not a failure under `fail_fast`.
- Once selected, `setup:` runs before the executable. Its failure still
  fails the task, with teardown exactly once after setup started.
- The sequence document's body is not a step, so `when:` has no meaning at
  the frontmatter root and is a typed error there.

`skip` retains its initialization-only placement until the iteration-skip
question below is resolved. A task wrapper `when` is distinct from a lifecycle
stack item's `when`. Add it to task-option parsing, not authored step state.

## `break`

A flow-control directive, valid in every lifecycle event and in a task's
`setup:` and `teardown:` stacks.

```yaml
- break                       # positional, no reason
- break: "recurring finding class"
- action:
      action: break
      reason: "recurring finding class"
      code: 2
```

| Parameter | Required | Meaning |
|---|---|---|
| `reason` | no | Recorded as the loop's outcome and shown in the summary. Positional. |
| `code` | no | Process exit code to use when this break ends the root invocation. Defaults to `0`. |

Semantics:

- `break` ends the innermost enclosing loop, whichever primitive owns it. A
  document inside a looped group ends the group's loop; a group inside a
  looped sequence ends the sequence's loop.
- It ends the current stack and records a pending break. The task that
  raised it completes normally: a running document finishes its terminal event
  and `finalize`, and the task's `teardown` runs exactly once. No further
  sibling task in the same iteration is admitted, so a `break` from the review
  task of a review, repair, commit group prevents the repair and the commit.
  The loop then stops without evaluating its condition or applying its
  `action`. The alternative, finishing every sibling first, is recorded under
  **Open questions** for the owner to veto.
- The first break captures its reason/code. A later cleanup failure, interrupt,
  or budget stop takes precedence over a successful break. Restrict authored
  codes to integers `0..255`; existing reserved runtime stop codes keep their
  meanings. A break before provider launch does not fabricate provider output.
- Whether an enclosing loop exists is a runtime fact. `break` with no
  enclosing loop raises a typed error at the moment it fires, in the same class
  as `resume` with no session. It is not a parse-time placement check.
- With nested loops only the innermost ends. The outer loop then evaluates its
  own condition as usual.
- When the loop that ended is the root invocation's, the process exits with
  `code`. When it is an inner loop, `code` is recorded but the run continues.

## `prep`

A flow-control directive, valid in every lifecycle event. Where `proxy` is a
hand-off, `prep` is a call: the target runs to completion and control returns
to the item after the directive.

```yaml
initialize:
    stack:
        - when: "stage == 'all'"
          action: { prep: "@prompts/_git/stage-all.md" }
```

| Parameter | Required | Meaning |
|---|---|---|
| `target` | **yes** | The document, group, or sequence to run. Positional. |
| `with` | no | Frontmatter overlay for the target, as for `proxy`. Key/value form only. |

Semantics:

- **Return channel.** Choose disk-only return: target output is rendered and
  reported under the child invocation but is not appended to the caller's
  `state.seq.outputs`. The target owns a fresh mutable runtime and output
  accumulator. Caller frontmatter, loop frames, and setters do not merge back;
  callers observe disk changes through fresh read functions or their next
  preparation. Capture explicit `with` values using the caller's event-time
  evaluator, preserving file-value provenance as for `proxy`.
  **Reader's note:** the draft simultaneously promised disk-only return and
  an output mutation. Removing the latter avoids hidden changes to the
  caller's `last(state.seq.outputs)` when a lifecycle helper runs.
- **Preparation.** The target is prepared exactly as a `proxy` target is: its
  own `initialize`, schema validation, and shell approval. In a sequence, that
  inventory includes statically known `prep` targets transitively without
  executing them. Existing static required includes must exist at that boundary;
  `prep` does not make missing static includes legal. Dynamic targets must pass
  the same target preparation and shell approval before launch, and cannot
  borrow approval for a different path or command. Use the shared target runner,
  not a recursive invocation of the CLI.
- **From `initialize`.** Proposed, but blocked on the approval-boundary
  question below. The caller's shell-free initialization guard must not be
  disabled or marked approved merely because a child was requested. The draft's
  claim that child approval alone resolves this is not a complete contract.
- **Failure.** A target that fails fails the caller's current event. The run
  follows the existing lifecycle error route with the child cause attached.
  In `failure` or `finalize`, do not recursively reenter that same event;
  preserve the original primary error and exactly-once cleanup. An outer task
  sees the resulting failure and applies its normal `fail_fast` policy.
- **Cycles.** `prep` chains obey the `proxy` cycle rule and hop limit. A
  target cannot `prep` or `proxy` an active ancestor. Track canonical target
  identities across the combined active call/handoff chain, preserving the
  proxy hop bound; returning releases the child's ancestry so separate later
  calls to the same helper are legal. A child's `break` cannot escape into a
  caller loop: `prep` creates a control-flow boundary.
- **Nesting with `proxy`.** A target may `proxy`; the proxied document then
  owns the rest of the target's run, and control still returns to the caller
  when that run ends.
- **Relation to `setup:`.** `setup:` holds actions; `prep` runs a primitive.
  `setup: [{ prep: "@x.md" }]` is the natural spelling when a task needs a
  whole prompt run first.

## `defer`

Contract only. Implementation waits for `rendezvous` scheduling and is out of
scope here.

- `defer` suspends the **root invocation**, whichever primitive fired it. A
  `defer` from a step suspends the enclosing sequence; from a task, the
  enclosing group and whatever contains it.
- Nothing runs while suspended: no later events, no later steps, no provider.
- On resumption the run continues at the point where `defer` fired. Earlier
  steps are not repeated.
- Parameters: `delay` (required, positional), `reason` (optional).
- Until the scheduler exists every `defer` raises `LifecycleDeferNotImplemented`,
  as today.

## State

### Execution limits shared by every primitive

Loops and `prep` must pass the same invocation budget, cancellation signal,
timeout accounting, and runaway protections through every provider launch,
retry, and proxy. A private `prep` variable store is not a fresh execution
budget. Preserve budget exits `76` (exhausted), `77` (blocked), and interrupt
exit `130`; none may be converted into a successful loop outcome. Unwind
active children and perform existing cleanup without launching more work.

### The `state` root

`state` is already the one reserved root every sequence author knows, and
`doc.*`, `state.*`, `ctx.*`, and `env.*` then read as four clean roots: what
the author wrote, what the runtime knows, what Claudine observed about the
host, and the process environment. `doc.loop` is a loop's configuration;
`state.loop` is what it produced.

The step's own `state` object is unchanged. Every other built-in moves under
one of two reserved keys inside it:

| Today | After |
|---|---|
| `state`, `state.name`, `state.<authored>` | unchanged |
| `previous` | `state.seq.previous` |
| `next` | `state.seq.next` |
| `outputs` | `state.seq.outputs` |
| `sequence_id` | `state.seq.id` |
| `_loop_count` | `state.loop.count` |
| `_loop_is_first` | `state.loop.is_first` |
| `_loop_is_last` | `state.loop.is_last` |
| `_loop_last_output` | `state.loop.last_output` |
| `_loop_last_exit_code` | `state.loop.last_exit_code` |

- `loop` and `seq` join the reserved authored-state keys: a step whose state
  declares either is a typed normalization error naming the step and key, and
  `set` may not target them.
- The name-coercion rule carries over unchanged for `{{ state }}`,
  `{{ state.seq.previous }}`, and `{{ state.seq.next }}`.
- `state.seq.outputs` also exists for standalone compose/inline-compose,
  preserving their existing output-history support. Sequence-only members
  (`id`, `previous`, `next`) are absent there. Within a sequence, neighbors
  refer to declared adjacent steps, even when skipped; they do not wrap across
  iterations. First/last missing neighbors remain null. Generated step IDs and
  indices stay stable across passes; iteration count distinguishes repeated runs.
- `state.loop` exists only inside a loop and names the innermost active loop.
  Restore the outer frame after the inner loop returns. Count is 1-based;
  `is_first` is `count == 1`; previous output/exit initially remain `""`/`0`.
  `is_last` cannot predict future file writes: define it as the final permitted
  pass (`count == effective max`), not a promise about condition completion.
  This intentionally replaces speculative condition evaluation; test and
  document the change so display hints cannot trigger early condition errors.
- Absent members follow the existing evaluator's missing-member/null behavior;
  an absent namespace is not automatically an unknown-root error. Preserve
  string name coercion for the three state-object paths listed above without
  coercing arbitrary objects or whole-value expression results.
- Remove the old injected aliases in the same migration. Keeping root
  `previous`/`outputs` aliases would preserve the collision this change fixes.
  Those authored root names become legal again; `state` remains reserved.
  Keep existing generated step fields, including `state.sequence_id`, for now.
  Update the shared reserved-key catalog, setter validation, interpolation,
  schema descriptors, examples, fixtures, and tests together. Search the whole
  repository for consumers; absence from shipped prompts alone is insufficient.

### Caller setters in shell tasks

A sequence `shell:` string can read `state`, template values, `doc.*`,
`ctx.*`, and `env.*`, but not a value the caller supplied as `key=value` or
`--set`, and authored step state cannot carry one either
(`agent: "{{ agent }}"` renders empty). After this change:

- caller setters are readable in a shell task as plain roots, resolved at
  preflight exactly as `doc.*` is, because they are early-binding values known
  before the first step;
- authored step state may interpolate caller setters and document frontmatter
  at normalization time.

Preserve the existing overlay precedence (including task `params` and reserved
runtime state); do not let caller values replace built-ins. Shell rendering
uses the same early-bound caller values as prompt preparation, with file-value
provenance intact. Reject references to `state.seq.outputs`, `state.loop`,
`current.*`, or mutable runtime setters in preapproved shell strings. Missing
caller values must produce a source-located error, not silently render empty.

### Path function anchors

**Reader's note:** a repository-root-only anchor would break established
file-reference rules. Preserve the shared resolver: explicit `./` and `../`
are document-relative; bare references probe the document directory then the
repository root; caller-supplied file values retain caller provenance. Prefixes
such as `@`, `~`, and vault references retain their own resolution rules.
This applies outside repositories too, without inventing a repository root.

The Darkmatter package's [document reference resolver](../../../darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs)
already shares file lookup and non-existing path construction. The claudine
implementation must use that contract and biscuit-file's [`FileReference`](../../../biscuit-file/lib/src/file_reference/mod.rs), not
add a separate path join. Resolve a file-index operation's input using its
provenance before modifying the filename; preserve the resulting parent.
`absolute` will additionally accept a missing local target by returning the
shared resolver's first candidate path, as other path-building functions do.
Do not turn permission errors, invalid references, or remote-fetch failures
into missing-file success. Existence checks and file reads keep their existing
return types and missing-file behavior; they do not all return absolute paths.
Update Darkmatter's function docs/catalog and focused tests with this change.

## Dry-run and shell tasks

`--dry-run` continues to execute `$( … )` expansions, whose output feeds
composition. It stops executing `shell:` tasks: each is rendered with its
approved bytes, reported as `rendered (dry-run): would run …`, and pushes an
empty placeholder in the dry-run report. A `--dry-run-shell` flag restores today's behavior
for a sequence that needs the side effect during rehearsal. It requires
`--dry-run`, is documented as mutating, and never launches providers.

A dry run visits each declared loop body once, labels repetition as unevaluated,
and does not apply loop mutations or iterate toward a file-dependent condition.
Render conditional tasks with their condition shown; do not predict a future
condition using missing provider results. Preserve static validation and approval.
Suppress lifecycle/setup/teardown side effects and `prep` execution; inspect
known helper targets for reporting only. Simulated output placeholders belong
only to dry-run reporting, not a successful runtime ledger. Expansion side
effects remain possible and must be stated in CLI help. Repeated iterations
must execute exactly the shell bytes approved at preflight.

## `stop` versus `skip` in shipped prompts

`stop` ends an event's stack; the provider still launches. `commit.md` ends
its `initialize` stack with `stop` when nothing is staged, so every commit
step after a loop has closed launches an agent that finds nothing to do. That
line becomes `skip`, and the lifecycle guide notes that `stop` inside
`initialize` is rarely what an opt-out wants.

## Migration

- Migrate `_loop_*` and root sequence metadata consumers throughout prompts,
  tests, READMEs, topic pages, and skill references to the new names. Change
  only built-in references, not authored keys coincidentally named `previous`.
- Replace `prompts/review-loop.md` with one implementation step followed by a
  looped serial review/repair group. Remove `in_loop` gates only when their
  replacement guards cover both `ready` and `recurrence`. Preserve explicit
  `spec`, `agent`, and commit-message parameters.
- Use this execution order (the previous unguarded example was incomplete):

  ```mermaid
  flowchart TD
      I[Implement if needed] --> R[Run review]
      R --> V[Read and validate the new review]
      V --> C{Recurring finding?}
      C -->|yes| H[Stop with a nonzero result for human review]
      C -->|no| Q{Ready?}
      Q -->|yes| D[Finish successfully]
      Q -->|no| F[Repair, stage, and commit]
      F --> M{Another iteration allowed?}
      M -->|yes| R
      M -->|no| E[Report exhaustion]
  ```

  The review target chooses its filename using the spec's current
  `review_iterations`. Capture that intended path for this pass and verify
  that this run produced it; do not reuse a stale ready review. Read `ready`
  and `recurrence` from that file after the review, requiring boolean values.
  Missing or malformed results fail before repair or staging. Guard each
  repair/stage/commit task so readiness or recurrence skips all three. Do not
  rely solely on the group's post-iteration condition. Recurrence takes
  precedence if both flags are true and uses a typed nonzero stop.
  The migration must include runnable YAML using the finalized break contract.
  The sketch below binds every value it reads; it is written in the current
  lifecycle grammar and is rewritten by `2026-09-21-lifecycle-ergonomics`
  before implementation:

  ```yaml
  ---
  $schema:
      spec: file(required;eager;match(**/*spec*.md)) -> the specification under review
      agent: string(required) -> the agent for every step
      max: number -> review passes before exhaustion
  max: 5
  # the review the current pass produced, derived from the spec's own counter
  latest_review: "{{ replace(spec, basename(spec), 'review-' + (frontmatter(spec, 'review_iterations') || 0) + '.md') }}"
  sequence:
      - name: implement
        prompt: "@prompts/_implement/implement-plan.md"
        when: "frontmatter(spec, 'implemented') != true"
        params: { spec: "{{ spec }}" }
      - name: review-repair
        group:
            execution: serial
            loop:
                until: "frontmatter(latest_review, 'ready') == true"
                max: "{{ max }}"
            tasks:
                - name: review
                  prompt: "@prompts/_reviews/feature-review.md"
                  params: { spec: "{{ spec }}" }
                  teardown:
                      - when: "frontmatter(latest_review, 'recurrence') == true"
                        action:
                            - warn: "a finding class recurred; stopping for a human"
                            - break: { reason: "recurring finding class", code: 2 }
                - name: repair
                  prompt: "@prompts/_implement/implement-suggestions.md"
                  when: "frontmatter(latest_review, 'ready') != true"
                  params: { spec: "{{ spec }}" }
                - name: stage
                  shell: git add .
                  when: "frontmatter(latest_review, 'ready') != true"
                - name: commit
                  prompt: "@prompts/commit.md"
                  when: "frontmatter(latest_review, 'ready') != true"
                  params: { agent: "{{ agent }}", message: "repairs from {{ basename(latest_review) }}" }
  ---
  ```

  `latest_review` is re-read at every task boundary, so after the review task
  writes its file and bumps `review_iterations`, the guards on repair, stage,
  and commit, and the group's `until`, all see the new review.
- Define the cap as review passes, not repair passes. With `max: 5`, at most
  five reviews and five repairs run; an unreconciled final repair still ends
  in exhaustion because no subsequent review verified it. The default shipped
  prompt must document this difference from the old five-repair/six-review list.

- The `looping.md` and `flow-control.md` topics already describe the intended
  behavior and mark it planned. Remove references naming this feature from
  those pages while reconciling them with the final design. The `sequences.md`
  "out of scope" list that names group loops is corrected, and all affected pages
  drop their "planned" markers as each piece lands.
- The route-drift fixture under `claudine/cli/tests/fixtures/` is re-derived
  and its hash refreshed whenever a shipped prompt in the `implement` route
  changes.

## Acceptance criteria

1. **Loop reproduction.** A Level 1 test runs the small looping document
   from **Evidence** directly and as the only step of a sequence, with a stub
   provider that records each prompt it receives, and asserts three prompts in
   both cases. The same test is repeated for a looped serial group and a
   looped sequence document with three steps, asserting the prompt order and
   the `outputs` shape.
2. **Outcome reporting.** Tests for each of the three loop outcomes on each
   primitive: condition met, `max` reached, and `break`. Each asserts the
   summary line, the exit code, and exactly-once finalization where document
   events exist, plus task teardown for group members.
3. **`when:` on steps and tasks.** A skipped step launches no provider, fires
   no event, appends nothing to `outputs`, and does not fail `fail_fast`; a
   `when:` at the sequence root is a typed error.
4. **`break`.** From a document inside a looped group; from a group inside a
   looped sequence; nested loops ending only the innermost; runtime error
   with no enclosing loop; `code` reaching the process exit only when the root
   loop ends.
5. **`prep`.** Return of control to the next stack item; disk-only return
   channel; no caller output or setter mutation; failure of the target failing
   the caller's event; cycle refusal; transitive preflight approval of the
   target's shell in a sequence; the approved initialization boundary; mixed
   proxy/prep cycles; legal sequential helper reuse; failure during cleanup;
   child cancellation and budget exhaustion without a budget reset.
6. **State.** Every `state.seq.*` and `state.loop.*` value resolves where
   defined and follows existing missing-member semantics elsewhere; a step whose state
   declares `loop` or `seq` fails normalization naming the step and key;
   `{{ state }}` and `{{ state.name }}` are unchanged; old injected aliases are absent; a caller setter is readable from a
   shell task and from authored step state.
7. **Path anchors.** Table-driven fixtures cover document/repository filename
   collisions, explicit relative paths, caller file provenance, prefixed and
   absolute references, missing targets, and operation outside a repository.
   Assert each function's own return type against the same selected file, not
   identical strings from boolean and path functions. Include native Windows,
   macOS, Linux, and WSL2 path cases through portable test fixtures.
8. **Dry-run.** A sequence with a `shell: git add .` step leaves the index
   untouched under `--dry-run` and stages under `--dry-run-shell`.
9. **Rehearsal.** The compact review-loop sequence runs to
   completion in a scratch repository with a stub provider, twice: once with an
   unimplemented two-phase plan, asserting both phases run, the spec is marked
   implemented, one review is not ready, one repair and commit run, and the
   next review closes the loop; once with an implemented spec, asserting the
   implement step is skipped and no commit step launches a provider after the
   loop closes.
   Also cover recurrence, missing/stale/malformed review files, and cap
   exhaustion. Assert no repair, staging, or commit after a ready or recurring
   review, including a review on the last allowed pass.
10. **Step status names the provider that ran.** A sequence whose steps
    resolve to different providers reports each step under its own provider,
    not the sequence document's hint.
11. **Shipped prompt contract.** The existing `shipped_prompt_contract`,
    `compose_caller_file_provenance`, `compose_initialize_acceptance`, and
    `shipped_prompt_route_drift` tests pass after migration, and the claudine
    skill's composition and lifecycle pages describe the new primitives.
12. **Docs are the record.** Reconcile changed flow-control, looping,
    sequence, composition, and state pages with the approved design; remove
    planned markers only for shipped pieces. Keep `defer` planned. A search of
    every page this feature touches for `features/`, `fixes/`, and date-named
    directories returns nothing; unrelated existing documentation debt, such
    as the error-architecture catalog link, is not an acceptance blocker for
    this feature.
13. **Boundary coverage.** Test condition success on the final permitted pass,
    invalid/default/overridden caps, no mutation after exhaustion, failed
    iterations with both fail-fast settings, skip then run on a later outer
    pass, inner count reset and outer-state restoration, loop-condition access
    to runtime state, and output publication after failed teardown. Include
    guard parse/evaluation failures and false guards over invalid static tasks.
14. **Bounded rehearsal and shared limits.** Dry-run a condition depending on
    an agent-created file and prove it terminates after one preview pass;
    verify no lifecycle/prep side effects and explicit shell opt-in. Test
    budget exhaustion inside a loop and helper, interruption during helper
    execution, and rejection of changed unapproved shell bytes.

Use the repository Test Toolkit and the claudine CLI process fixture with
private CWD/home, fake providers, and suppressed audio. No test may launch a
real provider, change the developer's index, or focus a terminal/browser.
Implementation validation uses `just test` and `just lint` in the package area;
run `just test-l2` only for affected integration behavior. This inline review
changes the specification only and does not claim those implementation checks
have passed.

## Out of scope

- Implementing `defer`; only its contract is fixed above.
- Parallel groups that loop. A looped group is serial in this feature; a
  parallel group with `loop:` is a typed error until specified.
- Persisted checkpoint and resume of a looped sequence, beyond what `defer`
  will need.
- Changing the `proxy` cycle rule or hop limit.
- Any change to the lifecycle stack grammar itself, which
  `2026-09-21-lifecycle-ergonomics` owns. That feature also rewrites this
  spec's examples into its grammar and adds `break` and `prep` to the action
  enumeration before this one starts.
- Moving the error facet enums and code catalog out of the completed
  real-errors feature and into the error-architecture topic, so that page
  stops linking into `features/_completed/`. It is the same documentation
  rule this feature is held to, found while sweeping the flow-control pages,
  and it is a content move that deserves its own change.

## Open questions

These choices require owner review before their dependent behavior is
implemented. Recommendations are not recorded as owner decisions.

### Skipping an iteration

The owner requested iteration-level `skip`, but the current grammar permits
it only during initialization, which opts out of the whole document.

- **Keep initialization-only `skip` and use task `when` guards (recommended).**
  Pros: preserves cleanup and placement contracts; solves the review workflow
  without a new control signal. Cons: does not support abandoning the middle
  of an iteration. Recommend this scope unless that additional use is required.
- **Extend `skip` to mean continue-to-next-iteration inside loops.** Pros:
  supports the stated goal directly. Cons: needs new placement, output,
  finalization, counter/action, and nested-loop rules, and coordination with
  the lifecycle grammar feature `2026-09-21-lifecycle-ergonomics`.

### What finishes after `break`?

The draft says both that the iteration completes and that the loop ends. For
review → repair → commit, completing all siblings would still perform work
that the stop was meant to prevent.

- **Complete the current task, then stop admitting siblings (recommended).**
  Pros: familiar loop exit behavior; cleanup remains reliable; no unwanted
  repair/commit. Cons: requires propagation from nested tasks to the owning
  loop. Break in setup suppresses that task's primary executable and still
  runs teardown; break in an already-running document allows its existing
  terminal/finalize route. This is the recommended design for implementation.
- **Complete all remaining siblings, then stop repetition.** Pros: closest to
  the draft's literal wording. Cons: every later task needs an independent
  guard, and an early stop can still cause side effects. If chosen, document
  it as stopping repetition only and retain those guards in every example.

### Can initialization call a helper that executes work?

Initialization currently precedes approval and is shell-free, including error
and cleanup routes. A helper that starts an agent or shell creates a new
execution boundary; merely calling its approval "separate" is insufficient.

- **Allow an independently prepared child invocation (recommended).** Pros:
  fulfills the prep-before-composition use case. Cons: requires explicit child
  approval state, transitive inventory where possible, and budget/cancellation
  sharing. The parent remains unapproved; the child initializes shell-free and
  cannot launch until its own preflight succeeds. On return, parent preflight
  still runs against current content. Test denial, child proxy, and both error
  cleanup paths. This preserves the existing boundary instead of bypassing it.
- **Allow `prep` only after parent preflight.** Pros: smaller implementation
  and simpler approval reasoning. Cons: cannot prepare inputs needed by the
  caller's composition; authors must add an earlier sequence task.

### Control flow inside parallel groups

Rejecting `loop` on a parallel group does not settle a child break targeting
an outer serial loop, or two concurrent helpers modifying the same files.

- **Reject cross-worker loop control (recommended).** Pros: deterministic;
  retains snapshot isolation. Cons: an outer-loop break from a parallel member
  is a typed runtime error. Permit member-local document loops and helpers
  with isolated state; their breaks stay local. Authors remain responsible
  for disjoint disk writes, as with existing parallel tasks.
- **Coordinate outer breaks across workers.** Pros: supports more workflows.
  Cons: needs cancellation, cleanup, output ordering, and conflicting exit-code
  rules for work already admitted; substantially expands this feature.

The review resolves output shape (flat serial entries), helper return data
(disk only), and alias migration (one coordinated removal) in the normative
sections above; those are no longer unresolved alternatives.

## Evidence

### Loop reproduction

A small document, a one-step sequence, and a stub provider that appends
the prompt text it receives to a log.

```yaml
# count.md
---
n: 1
loop:
    until: "n >= 3"
    action: "increment(n)"
---
Iteration {{ n }}
```

```yaml
# seq.md
---
sequence:
    - name: counting
      prompt: "@count.md"
---
```

| Invocation | Prompts the provider received | Exit |
|---|---|---|
| `claudine compose @count.md` | Iteration 1, Iteration 2, Iteration 3 | 0 |
| `claudine sequence @seq.md` | Iteration 1 | 0, step reported succeeded |

### Rehearsal of the draft sequence

The 21-step draft ran in a scratch repository with a stub provider that
recognizes each shipped prompt by its heading and edits files the way an agent
would. With the spec unimplemented: the plan step ran phase 1 of 2 and stopped,
review 1 was not ready, the repair and commit ran, review 2 was ready, and
every later review and repair step skipped without launching. With the spec
already implemented: the plan step skipped, review 1 was ready, and the loop
closed. In both runs every commit step after closure launched a provider that
found nothing staged.

A follow-up probe with three stub providers showed that a sequence honors
each prompt task's own `agent:` and a step's `params: { agent: … }` when no
provider flag is given, and that a command-line `--<provider>` overrides every
step. The step status lines report the sequence document's agent hint rather
than the provider that actually ran, so a mixed-provider sequence prints the
wrong name on every step.

The rehearsal also surfaced the reserved-key collision on `previous`, the
`loop` parameter colliding with the lifecycle key, the doc-relative anchor of
`decrement_file_index`, `absolute` rejecting a path that does not exist, and
caller setters being invisible to shell tasks. Each is addressed above. These are the draft author's reported observations;
this review checked source contracts but did not rerun the provider rehearsal.

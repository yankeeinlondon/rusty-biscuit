---
area: claudine
status: draft-spec
created: 2026-09-21
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
reviewed: false
clarified: false
prepares:
    - 2026-09-27-sequence-improvements
related:
    - 2026-09-22-lifecycle-events
---
# Lifecycle Ergonomics

When we first created lifecycle events in Claudine we just provided a dictionary of methods for _communicating_ the event to callers. Later we added the stack based approach which allowed for far greater range in terms of what could be done at these events as well introduce the ability to make the actions we take _conditional_.

In this feature we will focus on making the API surface that has developed over time more ergonomic. This will be a breaking change and it's important to know that we do not currently have production users so this change can and will be made without the need to support both API variants for an interim period.

Terms used throughout: a **lifecycle event** is one of the frontmatter keys Claudine fires at a fixed point of a composition run (`initialize`, `start`, `blocked`, `success`, `failure`, `finalize`, and the `loop:` gate). An **action** is one thing an event does (print a line, speak, run a shell command, change flow). A **flow-control directive** is the subset of actions that decide what the run does next (`stop`, `skip`, `error`, `retry`, `resume`, `proxy`, `defer`, and the planned `break` and `prep`) rather than producing a side effect.

## Package Ownership and Sequencing

Everything in this feature lands in **Claudine**: the grammar, the parser, the validator, the schemas, and the audio-scheduling change. No Darkmatter code changes.

The parser and action-model slice — `parse.rs`, `action_shape.rs`, `actions.rs`, `validate.rs`, `signatures.rs`, and `source_map.rs` under `claudine/lib/src/composition/lifecycle/` — must be written **lift-ready**: it may not import Claudine runtime types, specifically global settings, the provider layer, messaging, or TTS/Playa types. This is discipline, not something the compiler enforces; the point is that a later feature can lift the slice into Darkmatter as a unit without untangling it first.

That later feature is `2026-09-22-lifecycle-events`, an active Darkmatter draft that extracts the lifecycle engine into Darkmatter so that any Darkmatter document, not only a Claudine prompt, can carry lifecycle events. It is written entirely in the grammar this feature removes (`stack:`/`action:`) and promises "no grammar change during extraction". This feature is therefore its prerequisite, and the two specs record that in frontmatter:

- this spec carries `related: [2026-09-22-lifecycle-events]` and keeps `prepares: [2026-09-27-sequence-improvements]`;
- the `2026-09-22-lifecycle-events` spec gains `depends-on: [2026-09-21-lifecycle-ergonomics]`, its examples must be rewritten in the new grammar as one of its first tasks, and its "preserve behavior before adding" rule starts from the post-ergonomics grammar.

Editing that spec's frontmatter and flagging its examples as old-grammar is an **implementation task of this feature**, recorded in this feature's implementation log.

Two housekeeping facts established while clarifying ownership:

- **Single schema home.** Claudine's schemas live in `claudine/schemas/` and nowhere else. Filled-in old-grammar copies exist at `darkmatter/docs/schemas/claudine.yaml` and `darkmatter/docs/schemas/claudine-types.yaml`; both are **deleted** by this feature, and the `.dmls.toml` example in `darkmatter/docs/topics/schemas/dmls-schema-support.md` that points at them is repointed to `claudine/schemas/`. `claudine/schemas/claudine.yaml` is currently an empty file.
- **No Darkmatter `completed` event.** Darkmatter has no document-lifecycle notion today (its only "lifecycle" is the unrelated preflight stage), so the placeholder `claudine/schemas/partials/lifecycle.yaml` is wrong where it claims Claudine's `success` renames a Darkmatter `completed` event. The claim is removed when that file is rewritten under this feature.

## Ergonomic Shift

One of the first things we will do to improve the ergonomics is remove the dual shapes of "messages dictionary" _and_ "stack" and instead ONLY have a stack but because we only have one shape for lifecycle events. For example, what would have been defined as:

```yaml
start:
    message: "starting something"
    stack:
        - action:
            - shell: "git status"
```

Would now be defined as:

```yaml
start:
    - message: "starting something"
    - shell: "git status"
```

It should be obvious from this simple example that configuration has gotten _easier_. It is a single flat array surface instead of a nested structure. There is no need to explicitly state `stack` anywhere.

- although the example didn't illustrate it, every _step_ in the stack can add a conditional `when` clause just like we can today.
- similarly, the available _actions_ are unchanged from what is available in the stack now it's just that there is no need to express `action` explicitly.

## Stack Structure

Today the "stack" is an explicit element but because the stack is all we have in the future state, there is no need to have it be explicit. Most of the general rules today still apply for this feature update:

- (no change from current) actions consist of:
    - communication (say/speak, effect, message, notify, stderr, stdout, info, warn, success)
    - `set`, the lifecycle mutation of document state
    - the side-effect verbs Darkmatter provides (`set_frontmatter`, `http_post`, ...)
    - shell commands (_bespoke_)
    - flow control directives (retry, resume, proxy, ...)

- (no change from current) the late binding variables like `err`, `current`, and `timing` are still available in exactly the same way

### Execution model

The stack is an **ordered array and nothing else**. There is no concurrency between actions, no audio thread, and no bundle concept in the executor.

- Items run strictly in the order written. A `set` mutation is visible to every action after it (this is today's behavior: the executor is a plain ordered loop, and `set` commits against a pre-write snapshot).
- A **dictionary bundle** — a `when` item with sibling verb keys, or a whole event written as a dictionary (see [Dictionary Grammar](#dictionary-grammar)) — is **desugared at parse time** into an ordered action list. The order is fixed and documented:
    1. the `when` guard (if present)
    2. `set`
    3. communication verbs, in the executor's existing order: `stdout`, `stderr`, `info`, `warn`, `success`, `message`, `notify`, then the audio verbs `effect`, `say`
    4. `shell`
    5. Darkmatter side-effect verbs
    6. the single flow-control key, last
- The root-level dictionary form survives; under desugaring it costs nothing extra.
- Audio (`say`/`speak`, `effect`) is **published** to Playa in stack order and never blocks on playback. Playa preserves publication order and serializes playback. See [Background Audio](#background-audio).
- An event is **complete** when its non-audio work has finished and its audio has been published — never "when the audio has played".
- Audio drains across flow-control transitions: when `retry`, `resume`, or `proxy` fires, audio already published for the event keeps playing in queue order and the next flow state's audio queues behind it. There is no cancel API and no job-ID bookkeeping.
- The first flow-control directive that fires **ends the entire event**, wherever in the stack it sits — at the root, inside a `then` body, or inside a nested `else`. Nothing after it in any enclosing list runs.

### Flow-control placement

Because a directive ends the whole event, actions written after an unconditional directive can never run. Claudine treats that as an authoring error rather than silently truncating:

- The rule is **list-local**. Within any one list — the event root, a `then` body, or an `else` body — an item that unconditionally fires a directive must be the **last** item in that list. Violations are a parse error, `LifecycleActionOrder` (today this error covers ordering within a single item; its meaning is widened to ordering within a list).
- A `when` item **never counts as unconditional**, even when both its `then` and `else` bodies end in a directive. Items may follow it. Claudine performs no cross-branch reachability analysis; the author is trusted to know what the branches do.
- More than one flow-control key in a single dictionary bundle is the parse error `LifecycleMultipleLifecycleActions`.
- The rule applies unchanged to the planned `break` and `prep` directives.

```yaml
failure:
    - when: "err.is_throttled"
      then:
          - info: "throttled; retrying"
          - retry: 3
      else:
          - warn: "not a throttle: {{ err.msg }}"
    - message: "failure handled"      # legal: the when item is conditional
```

Enforcement is **runtime-only** (Claudine's parser, which runs before anything executes). The schemas describe shapes and cannot express "must be last" (see [Expressiveness limits](#expressiveness-limits)), so DMLS, the Darkmatter language server, will not flag an unreachable action in the editor under this feature. Editor parity is deferred to the descriptor-driven DMLS mechanism that `2026-09-22-lifecycle-events` designs.

In addition to NOT having the "stack" property, in this feature's release we will also get rid of the `action` property. For actions which you don't want to put behind a conditional expression then the actions can be placed directly at the root level of the stack:

```yaml
success:
    - message: "starting something"
    - shell: "git status"
```

But if you want to add conditionals we still provide the `when` property and we add the `else` property. The `when` property behaves in exactly the same fashion as it did before except that it uses `then` instead of `action` as the aggregation point for the actions which are being wrapped in that conditional block:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - info: "it is summer!"
          - message: "we're starting something in the summer"
          - shell: "run-summer-program"
```

This is the normal mode of defining a conditional block but there are two other variants that exist:

1. Dictionary Grammar
2. Long Form

### Dictionary Grammar

The backbone of the stack is an array which preserves a discrete order which is important for the effective operation of the stack. However, there are _leaf_ nodes of the stack which can opt to use a key/value shape defining their actions. As an example:

```yaml
start:
    - message: "hi"
    - when: "ctx.season == 'summer'"
      message: "the heat is overwhelming"
      say: "do you have any water?"
```

In this example the `when`, `message`, and `say` properties are bundled into a group. The bundle is desugared at parse time into the canonical order given in [Execution model](#execution-model): the `when` guard is evaluated first and, if it is true, the remaining verbs run in that fixed order (`message` before `say` here). The author does not choose the order; that is the trade for the compact spelling.

This grammar is compact but has some limitations:

- the user can not specify the ordering of operations; the canonical order applies
- each verb can appear at most once (you can't call `say` twice because there's only one `say` key)
- no long form actions are allowed inside a bundle
- at most one flow-control key, and it always runs last
- `then` and `else` are **not** permitted on a dictionary item. If you need an `else`, use `then` for the true branch instead of sibling verbs.

One extra key is permitted as a sibling: `no_error: true`, which applies to every action in the item. (`no_error` means a side-effect dispatch failure — a channel, TTS, or shell action that evaluated fine but whose effect failed — is logged but does not stop the stack or change the composition outcome. It never suppresses an expression-evaluation error.)

```yaml
success:
    - when: "ctx.notify_team"
      message: "done: {{ title }}"
      shell: "post-summary.sh"
      no_error: true
```

This grammar can also be used at the root of the lifecycle event. That means that the following is valid:

```yaml
start:
    message: "hi"
    say: "nice work"
    shell: "doit"
```

Here the event is a single bundle: after desugaring it runs `message`, then `say`, then `shell`, and the event is complete when `shell` has finished and the audio has been published. A root-level dictionary follows the same rules as a bundled item: each verb at most once, no long form, at most one flow-control key (which runs last), `no_error` permitted, `then`/`else` not permitted.

### Long Form

This is not a real change in behavior, the current implementation already supports the idea of a long form action too. The basic idea is that our shorthand syntax of: `{command}: {param}` works very well for most cases because almost all of the actions really only _require_ a single parameter. However, many actions offer optional parameters that give the caller greater control over what the action does.

`no_error` is a parameter of **any** long-form action map, for example `- shell: { command: "post-summary.sh", no_error: true }`. That, and the dictionary-item sibling described above, are its **only two homes**; it is not accepted beside `then`/`else` on a `when` item.

A good example of this is that all flow control directives that move to another flow state provide an optional `with` parameter that allows the Frontmatter state of the next flow state to be prepared. This is the same `with` that `proxy` already accepts: an overlay applied to the target of the transition, for that transition only, never written to disk and never merged back into the caller. `prep` takes it too. `break` and `stop` do not, because neither has a next flow state to prepare.

- the default behavior for flow-state transitions is to move the current state exactly to the new flow-state (which might be the same prompt, a different one, or a sequence)
- the default behavior is good for a lot of flow-state transitions but it is very common that a caller will want to mutate state slightly for the next flow state.
- an example of this is when an agent hits an error of some sort on a prompt and you want to **retry** the action but you want the prompt to know that it's not the first time this has been tried and what the error was last time it happened:

```yaml
failure:
    - retry:
        with:
            reason: "{{ err.msg }}"
        max: 3
```

In the shorthand of a retry we would have done something like:

```yaml
failure:
    - retry: 3
```

The short form is _actionable_ but all it is able to express is how many times the retry logic should be tried before giving up.

By contrast, the long form allows us far greater expression and control:

- in our example we again set the maximum retries to 3
- but then we also set the `reason` frontmatter state to the _reason_ why the prior run failed allowing the conditional blocks and interpolation on the page to respond appropriately when the reason property is populated.

### Else Block

In this feature we will introduce an `else` key that pairs with `when` on the **same item**: `then` holds the actions for a true condition, `else` the actions for a false one. This allows the simple if/else logic that is fairly common for lifecycle events:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - info: "it is summer!"
          - message: "we're starting something in the summer"
          - shell: "run-summer-program"
      else:
          - info: "it is not summer"
```

Rules:

- `then` and `else` are keys of the `when` item. There is no stand-alone `- else:` item.
- `else` requires `then`. A `when` item either has `then` (optionally with `else`) or has sibling verb keys (the dictionary grammar), never both.
- A `then` or `else` body accepts exactly what an event accepts — a list, or a dictionary bundle — and does so recursively.
- These are parse errors, reported as `LifecycleStackInvalidShape`: an empty `then` or `else`; a `when` with neither `then` nor sibling actions; `then` or `else` on an item with no `when`.

A dictionary bundle as a body:

```yaml
failure:
    - when: "err.category == 'cap'"
      then:
          warn: "usage cap hit"
          proxy: "@prompts/feature-codex.md"
      else:
          - warn: "{{ err.msg }}"
```

But let's explore a config with a wrinkle:

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - info: "it is summer!"
          - message: "we're starting something in the summer"
          - shell: "run-summer-program"
      else:
          - info: "it is not summer"

    - message: "all is well that ends well"
```

In this example either the `then` or `else` block will be executed but in BOTH cases the final `message` will be executed because it is not conditional and not contained by either block of the conditional blocks.

### Conditional Nesting

Currently conditional blocks can not nest but with this feature we will start to allow nesting. The limit is **five** conditional levels; a sixth is a parse error (`LifecycleStackInvalidShape`). Five is generous — the most common requirement is a second level — and it is also what the schema can carry: `SimplifiedSchema`, Darkmatter's schema language, has no recursion, so `lifecycle.yaml` unrolls the `when`/`then`/`else` item shape five levels deep by hand (see [Schemas](#schemas)).

The syntax for nesting is keeping in line with the normal grammar: a `then` or `else` body is a list, and a list may contain further `when` items.

```yaml
start:
    - when: "ctx.season == 'summer'"
      then:
          - when: "ctx.month == 'July'"
            then:
                - message: "damn it is hot, and it is July!"
            else:
                - message: "damn it is hot; at least it is not July!"
      else:
          - message: "not summer"
```

There is no `elif` or `else_when` key. An else-if chain is a `when` nested inside `else`:

```yaml
failure:
    - when: "err.code == 'cap.rate_limit'"
      then:
          - retry: 3
      else:
          - when: "err.category == 'cap'"
            then:
                - proxy: "@prompts/feature-codex.md"
            else:
                - warn: "unrecoverable: {{ err.msg }}"
```

Each `when` nested this way counts one level toward the limit of five.

## Schemas

The schema support that Darkmatter provides now is fairly comprehensive and the DMLS language server is a great aid in helping authors create _valid_ Darkmatter documents. However, to date, we have not yet brought in the Claudine schemas but with this feature release we will bring in schemas for Claudine (not just the lifecycle properties).

### Darkmatter Provisioning

What Darkmatter implements today, and what this feature builds on:

- A document can name its schema explicitly with the `$schema` frontmatter property: inline (the schema written in the frontmatter), as a path to a YAML file, or as a **bare name** (`$schema: claudine.yaml`) resolved against the schema roots below.
- Darkmatter discovers schema roots by an **ancestor walk**: starting from the document's folder it looks for a `schemas/` directory in each parent up to the repository root (when composing) or the editor workspace root (in DMLS).
- Only files of `kind: trigger-schema` **auto-apply**. A trigger-schema is an envelope with a `match:` block and a `$schema` payload; when the match conditions hold for a document, the payload is applied to it. Match conditions are frontmatter-only: `property: <type-expr>` tests, the combinators `all`/`any`/`none`/`min-match`, and `$path` globs. Body-content matching is rejected. DMLS applies matched triggers and reports diagnostics against them.
- Plain `kind: schema` files are **import libraries**. The discovery scan silently ignores them; they reach a document only when named by `$schema` or imported by a trigger's payload.
- The code spells the kind `trigger-schema`; Darkmatter's docs spell it `schema-trigger`, which the parser does not accept. Claudine follows the **code spelling** until Darkmatter renames one or the other.

### Claudine Schema Structure

Five files, two of them type libraries and three of them triggers whose payloads import from the libraries:

| file path             | kind             | description                                                                                                                                                                                                                                                                             |
|-----------------------|------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `action.yaml`         | `schema`         | types-only library: the action enumeration (every verb and directive, planned ones included), each with its short and long form shapes                                                                                                                                                  |
| `lifecycle.yaml`      | `schema`         | types-only library built on `action.yaml`: the full shape of an event — a list of items or a dictionary bundle; the `when`/`then`/`else` item; the bundle as a pattern-keyed object; nesting unrolled to depth 5                                                                          |
| `claudine.yaml`       | `trigger-schema` | matches on `any:` of the six event keys (`initialize`, `start`, `blocked`, `success`, `failure`, `finalize`) or `agent`/`model`; payload is the catalog of Claudine frontmatter properties below                                                                                         |
| `inline-compose.yaml` | `trigger-schema` | matches `prompt: string(required)` with `none: { sequence: any }`; payload is the shape of an inline-compose document                                                                                                                                                                    |
| `sequence.yaml`       | `trigger-schema` | matches `sequence: any`; payload is the shape of a sequence document, written to the shape `2026-09-27-sequence-improvements` defines (`when:` on a step or task, `loop:` on a group and at the root, `loop` and `seq` reserved) so that spec's examples validate the day the schema lands |

The `claudine.yaml` payload describes the frontmatter properties common to every Claudine prompt, taken from `claudine/docs/topics/frontmatter-properties.md`:

- composition core: `prompt`, `agent`, `model`, `interactive`, `$schema`, `last_updated`, `hash`, `fail_fast`, `sequence`, `loop`, `title`, `description`
- the six lifecycle events, typed from `lifecycle.yaml`
- timeouts: `timeout`, `step_timeout`, `timeout_warn`, `step_timeout_warn`
- run controls: `exit_expressions`, `guard_settings`, `operation`, `yolo`, `verbosity`, `mode`
- generated sequence/loop overlays: `state`, `previous_state`, `next_state`, `is_first`, `is_last`, `step`, `total_steps`; the `_loop_*` values become `state.loop.*`

**Open ruling:** the linking-only keys (`name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns`) describe how a prompt is linked into a provider as a skill, command, or agent rather than how it composes. Whether they are excluded from `claudine.yaml` or given a separate linking schema is not yet decided; see [Open Questions](#open-questions).

### Expressiveness limits

`SimplifiedSchema` can express what the grammar looks like but not everything the grammar requires:

- **Can:** unions (`anyOf`) of inline objects as array items; named types via `Name@this` and imports; pattern keys (`<string>`) with `min-keys`/`max-keys` for the bundle shape.
- **Cannot:** recursion (schemas are a DAG, so nesting is unrolled per level); an ordering constraint such as "the flow-control item must be last" (arrays support only `min`/`max`/`unique`, and the planned tuple form puts the spread last); mutual exclusion or at-most-one-of over keys.

Hence the division of labor: the schemas describe shapes, and Claudine's parser remains the **sole enforcer** of placement (directive last, list-local) and cardinality (one flow-control key per bundle, each verb at most once, `then`/`else` not beside sibling verbs). No Darkmatter code changes.

### Schema spike

The first task of the schema work — not a precondition for anything else — is a spike of about half a day: write the two type libraries and one trigger, then validate three fixtures (a flat stack, a root-level dictionary, and a nested `when` inside `then` with an `else`) through `md schema validate`, `md schema triggers`, and DMLS. Three known traps it must prove out: local type references need the `@this` suffix; the resolver strips `required` from reused named types; and the `trigger-schema`/`schema-trigger` envelope spelling drift.

### Location

All five files live under `claudine/schemas/`, which the ancestor walk discovers for any document under `claudine/`. A document elsewhere reaches the schema in one of three ways: a `schemas/` directory at the repository root that contains or links the files; a bare-name pointer `$schema: claudine.yaml` resolved against the schema roots; or a `.dmls.toml` `[schema.extensions]` entry naming `claudine/schemas/`.

## Background Audio

Today we have both TTS and sound effects which are played through the host's audio system, via Playa, Claudine's audio playback layer.

The awkwardness is not latency. `say`/`speak` and `effect` already block only on **publication** to Playa's per-user queue (milliseconds; on a TTS cache miss a sequence slot is reserved and a separate preparation process is spawned), and Playa preserves publication order. The ergonomic problem is only the ordering API:

- because it can be useful to play a sound effect to get a user's attention _and then_ have the TTS speak afterwards, we added `say_first` and relied on a built-in effect-before-`say` ordering (`audio_phases`), with `say_first` as the way to reverse it.
- to understand `say_first` the user is required to know too much about how Claudine and Playa order things.

This feature therefore:

- **removes `say_first`** and the fixed `audio_phases` ordering. If you care about order, order the array: `effect` before `say` plays the effect first; `say` before `effect` speaks first.
- publishes audio to Playa **in stack order**; Playa serializes playback. An event is complete when its non-audio work has finished and its audio has been published, exactly as stated in [Execution model](#execution-model).
- deletes the top-level notification-field execution path (the fixed-order dictionary code in the executor) once the dictionary form desugars into the stack, so there is one execution path.

```yaml
success:
    - effect: "chime"
    - say: "the build is green"
    - message: "build complete"
```

### Pre-compilation of TTS assets (still under clarification)

Independent of the ordering change is the idea of warming the TTS cache when a page loads: evaluate every `say`/`speak` the page defines, and for phrases whose text is static or interpolates only early-binding state (`ctx`, document globals, frontmatter) — not late-binding `err`/`current`/`timing` — produce the audio asset ahead of time so that _if_ the phrase is needed there is almost no latency. Phrases that depend on late-binding values cannot be pre-compiled.

None of the following is decided, and this spec does not decide it:

- where the cache lives and how entries are keyed (phrase text, voice, provider, and what else)
- which TTS providers produce an intermediate audio file that can be cached, and how the warm-up interacts with Playa's existing `reserve_preparation` path
- what to do for providers such as macOS `say` that produce no intermediate file: whether pre-computing makes sense at all
- cleanup: when cached assets expire and who removes them

## Loop Lifecycle

The `loop:` block mixes two kinds of keys. Its iteration controls — `while`/`until`, `action`, `max`, `fail_fast`, and the rest — are owned by `2026-09-27-sequence-improvements`, which also owns when the condition is checked, what `action` mutates, the three loop outcomes, and `break`. This feature says one thing about `loop:`: its **lifecycle concerns use the same grammar as every other event**. Because the block is a dictionary, its lifecycle concerns are read as a root-level dictionary bundle — the canonical order, each verb at most once, a single flow-control key last, and no `stack:` or `action:` aggregator — and they fire on every gate pass, including the terminal pass that exits the loop.

```yaml
loop:
    while: "iteration < max_iterations"
    action: increment(iteration)
    stderr: "loop gate"
    info: "finished iteration {{ iteration }} of {{ max_iterations }}"
```

## Hand-off: prepare `2026-09-27-sequence-improvements`

The sequence-improvements feature is sequenced **after** this one because every
lifecycle example it contains is written in the grammar this feature removes,
and two of the directives it adds (`break`, `prep`) belong in the action
enumeration this feature defines. Its implementation must not start until the
following has been done, and doing it is the last task of this feature:

1. **Rewrite every lifecycle example in that spec** into the grammar defined
   here: no `stack:`, no `action:`, same-item `then`/`else` for conditionals, and
   the dictionary grammar where it reads better. The semantics the examples show
   must not change; only their spelling. The canonical bundle order in
   [Execution model](#execution-model) is now fixed, and it is what reviewers
   check each rewritten dictionary example against: if the old example's
   ordering differs from the canonical order, the rewrite must use the list
   form.
2. **Add `break` and `prep` to `action.yaml`** with both a short and a long
   form, using the parameter tables in that spec (`break`: `reason`, `code`;
   `prep`: `target`, `with`). Add `defer` if it is not already enumerated, so
   the schema names every directive the flow-control page documents, planned
   ones included.
3. **Confirm the placement rules here cover the new directives.** The rule is
   list-local: within any one list, an item that unconditionally fires `break`
   or `prep` must be last, exactly as for any other directive, and a `when`
   item never counts as unconditional. `break` inside a nested conditional is
   legal and ends the innermost enclosing loop. The sequence spec's `break` and
   `prep` sections still say a directive "ends the current stack", which
   describes the old silent truncation; restate them against the entire-event
   rule and the list-local error so a reader does not infer truncation.
4. **Keep `with` as the one overlay parameter.** Every directive that moves
   to another flow state (`retry`, `resume`, `proxy`, `prep`) takes `with`,
   with `proxy`'s existing semantics: an overlay on the target, for that
   transition only, never persisted, never merged back. There is no separate
   `use`. `break` and `stop` take no overlay.
5. **Write `sequence.yaml` to the shape the sequence spec defines**, not to
   today's shape: `when:` on a step or task, `loop:` on a group object and at
   a sequence document's root, and `loop` and `seq` as reserved authored-state
   keys. Otherwise DMLS rejects every example in that spec the day the schema
   lands.
6. **Keep the "Loop Lifecycle" section above to one job:** the `loop:`
   block's lifecycle concerns use the same grammar as every other event.
   Everything about when the loop condition is checked, what its
   `action` mutates on each primitive, the three loop outcomes, and `break`
   is owned by the sequence-improvements spec and is not restated here.
7. **Migrate the `_loop_*` reads** in shipped prompts to `state.loop.*` in the
   same pass that rewrites their `stack:`/`action:` blocks, so those files are
   swept once. The `in_loop` gates in `feature-review.md`,
   `implement-suggestions.md`, and `implement-plan.md` stay until `when:` on
   steps exists; only their spelling changes here.
8. **Record the rewrite** in that spec's frontmatter (`grammar: lifecycle-ergonomics`
   or similar) and in this feature's implementation log, and set its
   `depends-on` as satisfied.

The flow-control and looping topic pages already describe `break`, `prep`, and
uniform loops as planned. Their examples are also in the old grammar and are
rewritten under this feature's normal documentation duty, not deferred to the
sequence work.

## Open Questions

Decisions this spec does not make. Each needs a ruling before the affected work is planned.

1. **TTS pre-compilation.** Everything listed under [Pre-compilation of TTS assets](#pre-compilation-of-tts-assets-still-under-clarification): cache location and keying, which providers produce an intermediate file, whether macOS `say` can be pre-computed at all, and cleanup. The ordering decisions above do not depend on any of it.
2. **Linking-only keys in `claudine.yaml`.** The keys that describe how a prompt links into a provider (`name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns`) are either excluded from `claudine.yaml` or given a separate linking schema. Excluding them means DMLS flags them as unknown on every linked prompt; including them blurs a composition schema with provider-linking concerns.
3. **Long-form parameter tables.** Only `retry` is shown in this spec (`with`, `max`). `action.yaml` needs the full parameter set for every verb and directive — communication verbs (`route`, ...), `shell` (`command`, `no_error`, ...), `set`, every Darkmatter side-effect verb, and each directive (`backoff`, `delay`, `max_attempts`, `with`, `break`'s `reason`/`code`, `prep`'s `target`/`with`). The current lifecycle topic page and the flow-control reference are the sources; the tables have not been consolidated.
4. **Acceptance criteria and test strategy.** None exist yet. At minimum the plan needs: parser fixtures for every rule named in this spec (canonical desugaring order, list-local placement, each `LifecycleStackInvalidShape` case, depth 5 versus 6), an executor test that audio publishes in stack order and the event completes before playback, a drain test across `retry`, and the three schema fixtures from the spike validated in both `md` and DMLS.
5. **Migration sweep.** Every shipped prompt, `docs/` topic page, and `.claude/skills/` file that spells a lifecycle event in the old grammar (`stack:`, `action:`, `say_first`, notification-field dictionaries with fixed ordering assumptions) must be rewritten. The inventory has not been taken, and whether the sweep is one commit or per-area commits is undecided.
6. **Ordered lifecycle concerns inside `loop:`.** The `loop:` block is a dictionary, so this spec reads its lifecycle concerns as a dictionary bundle. Whether a loop ever needs the ordered list form (and if so, under which key, since `stack:` is gone) has not been ruled on; today's engine only supports the bundle-style keys plus the removed `stack:`.

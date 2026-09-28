---
area: claudine
status: finalized-spec
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
    review_note: string -> a note qualifying the review, such as what process stood in for a separate review
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    clarified_by: string -> the agent and model that ran the clarification
    needs_rulings: boolean -> whether rulings remain before planning and implementation
    required_rulings: string[] -> the rulings still required when needs_rulings is true
    references: object -> map of document path to a 1–2 sentence description of what it is
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: claude/fable (clarification review)
reviewed_on: 2026-09-28
review_note: "the clarification process served as a review"
clarified: true
clarified_by: claude/fable
needs_rulings: false
prepares:
    - 2026-09-27-sequence-improvements
    - 2026-09-28-recursive-schema-types
related:
    - 2026-09-22-lifecycle-events
    - 2026-09-28-recursive-schema-types
references:
    spike-results.md: >-
        Measurements, findings, and verdicts of the 2026-09-28 schema scale spike.
    spike/README.md: >-
        How to reproduce the spike.
    spike/gen-action-schema.py: >-
        Generator for the action schema's side-effect and expression-function entries,
        retained as the mechanism for switching full typing on later.
    spike/schemas/lifecycle.yaml: >-
        The depth-5 unrolled reference schema the interim shape is derived from.
    matrix.md: >-
        Functionality and ownership matrix across Claudine, Darkmatter, and the
        supporting packages as of 2026-09-21.
---
# Lifecycle Ergonomics

When we first created lifecycle events in Claudine we just provided a dictionary of methods for _communicating_ the event to callers. Later we added the stack based approach which allowed for far greater range in terms of what could be done at these events as well introduce the ability to make the actions we take _conditional_.

In this feature we will focus on making the API surface that has developed over time more ergonomic. This will be a breaking change and it's important to know that we do not currently have production users so this change can and will be made without the need to support both API variants for an interim period.

Terms used throughout: a **lifecycle event** is one of the six frontmatter keys Claudine fires at a fixed point of a composition run (`initialize`, `start`, `blocked`, `success`, `failure`, `finalize`). A **stack** is any place the lifecycle action grammar is accepted: the six events, the `loop:` block's `gate:` (see [Loop Lifecycle](#loop-lifecycle)), and a sequence task's `setup:` and `teardown:`, which the same lifecycle parser reads today. Everything this spec says about an event's grammar holds for every stack. An **action** is one thing a stack does (print a line, speak, run a shell command, change flow). A **flow-control directive** is the subset of actions that decide what the run does next (`stop`, `skip`, `error`, `retry`, `resume`, `proxy`, `defer`, and the planned `break` and `prep`) rather than producing a side effect. **Document prepare** is the existing `prepare_document` boundary: the step that composes a document and runs before `initialize` fires; the TTS pre-warm pass in [Pre-compilation of TTS assets](#pre-compilation-of-tts-assets) runs there.

## Package Ownership and Sequencing

Everything that defines the grammar lands in **Claudine**: the parser, the validator, the schemas, the audio-scheduling change, and the TTS pre-warm pass. Two other packages are touched, in bounded ways:

- **biscuit-speaks** gains the uniform `prepare` API and the cache-consult fix described in [Pre-compilation of TTS assets](#pre-compilation-of-tts-assets). That is a real library change with its own tests and documentation duty, not a Claudine-side workaround.
- **Darkmatter** gains **no lifecycle machinery** — no engine, no grammar knowledge, no new parser. What does change on the Darkmatter side is test and mirror material that must move with the grammar, and it is part of this feature's [Migration Sweep](#migration-sweep): the static lifecycle-layout mirror in `darkmatter/dmls/src/diagnostics/nested_span.rs` and its parity test, the `initialize.stack[0].when` pointer tests in `darkmatter/dmls/src/overlay/frontmatter.rs`, the `mapping_only_corpus.rs` corpus test, the `sequence_descent/implement-plan.md` fixture, and the deletion of the Darkmatter-side schema copies named below.

The parser and action-model slice — `parse.rs`, `action_shape.rs`, `actions.rs`, `validate.rs`, `signatures.rs`, and `source_map.rs` under `claudine/lib/src/composition/lifecycle/` — must be written **lift-ready**: it may not import Claudine runtime types, specifically global settings, the provider layer, messaging, or TTS/Playa types. This is discipline, not something the compiler enforces; the point is that a later feature can lift the slice into Darkmatter as a unit without untangling it first.

That later feature is `2026-09-22-lifecycle-events`, an active Darkmatter draft that extracts the lifecycle engine into Darkmatter so that any Darkmatter document, not only a Claudine prompt, can carry lifecycle events. It is written entirely in the grammar this feature removes (`stack:`/`action:`) and promises "no grammar change during extraction". This feature is therefore its prerequisite, and the two specs record that in frontmatter:

- this spec carries `related: [2026-09-22-lifecycle-events]` and keeps `prepares: [2026-09-27-sequence-improvements]`;
- the `2026-09-22-lifecycle-events` spec gains `depends-on: [2026-09-21-lifecycle-ergonomics]`, its examples must be rewritten in the new grammar as one of its first tasks, and its "preserve behavior before adding" rule starts from the post-ergonomics grammar.

Editing that spec's frontmatter and flagging its examples as old-grammar is an **implementation task of this feature**, recorded in this feature's implementation log.

Two housekeeping facts established while clarifying ownership:

- **Single schema home.** Claudine's schemas live in `claudine/schemas/` and nowhere else. Filled-in old-grammar copies exist at `darkmatter/docs/schemas/claudine.yaml` and `darkmatter/docs/schemas/claudine-types.yaml`; both are **deleted** by this feature, and the `.dmls.toml` example in `darkmatter/docs/topics/schemas/dmls-schema-support.md` that points at them is repointed to `claudine/schemas/`. `claudine/schemas/claudine.yaml` is currently an empty file.
- **No Darkmatter `completed` event.** Darkmatter has no document-lifecycle notion today (its only "lifecycle" is the unrelated preflight stage), so the placeholder `claudine/schemas/partials/lifecycle.yaml` is wrong where it claims Claudine's `success` renames a Darkmatter `completed` event. The claim disappears when that placeholder is replaced under this feature ([Action Inventory](#action-inventory), rule 9).

**Darkmatter follow-on: recursive named types (sequenced after this feature).** The schema scale spike ([Schema spike (run 2026-09-28)](#schema-spike-run-2026-09-28)) found that `SimplifiedSchema` substitutes every named type structurally — there is no `$defs`/`$ref` — so an unrolled lifecycle schema clones the whole action table once per reference, and that an array cannot carry a union-typed named item. The remedy is recursive named types in Darkmatter's schema resolver, filed as the Darkmatter feature `2026-09-28-recursive-schema-types`, which declares this feature as its `depends-on`, and estimated from source, with nothing built, at 13–20 engineer-days (likeliest 16), plus 1–1.5 days to lift the array-of-union restriction in the same change, which is cheap together and expensive alone. The resolver change itself is small; the bulk is the twenty-two consumer sites that walk the resolved schema. Its design keeps every existing schema lowering byte-identically by inlining acyclic types exactly as today and emitting `$defs`/`$ref` only for cyclic types and unions under `[]`. That feature is the **target state** for Claudine's schemas and runs **after** this one: this feature ships the interim shape described under [Schemas](#schemas) and leaves the generator and spike fixtures in place; switching Claudine's schemas to `item[]@this` recursion, restoring full typing of the generated verbs, and making the nesting limit parser-only are tasks of `2026-09-28-recursive-schema-types`, not of this feature. Four Darkmatter defects the spike exposed are filed as non-blocking, in that feature or as fixes: an array over a union-typed named type is rejected; a diagnostic's line and column point at the top-level key rather than the offending node; `required` is stripped from a reused named type (a separate half-day decision, not bundled with the recursion work); and two valid-YAML spellings — a block sequence whose `-` sits at the parent key's indentation, and a plain scalar folded across lines — crash the schema loader. Two cheap pieces of Darkmatter pre-work are recommended and are **not** part of this feature; they belong to `2026-09-28-recursive-schema-types` or to fixes raised ahead of it: share type tables behind an `Arc` in the resolver instead of cloning them per expansion (about half a day; it cuts resolve time directly) and fix the two loader crashes. A DMLS resolved-schema cache keyed on `$schema`, registry, and dependency hashes (2–3 days) would fix per-keystroke re-resolution only, not size.

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
    - the side-effect verbs and expression functions Darkmatter provides (`set_frontmatter`, `http_post`, `file_exists`, ...)
    - shell commands (_bespoke_)
    - flow control directives (retry, resume, proxy, ...)

- (no change from current) the late binding variables like `err`, `current`, and `timing` are still available in exactly the same way

The full list, with each verb's short and long form, is the [Action Inventory](#action-inventory).

### Execution model

The stack is an **ordered array and nothing else**. There is no concurrency between actions, no audio thread, and no bundle concept in the executor.

- Items run strictly in the order written. A `set` mutation is visible to every action after it (this is today's behavior: the executor is a plain ordered loop, and `set` commits against a pre-write snapshot).
- A **dictionary bundle** — a `when` item with sibling verb keys, or a whole stack written as a dictionary (see [Dictionary Grammar](#dictionary-grammar)) — is **desugared at parse time** into an ordered action list. The order is fixed and documented:
    1. the `when` guard (if present; only a list item can carry one — a root-level dictionary never does)
    2. `set`
    3. communication verbs, in the executor's existing order: `stdout`, `stderr`, `info`, `warn`, `success`, `message`, `notify`, then the audio verbs `effect`, `say`
    4. `shell`
    5. Darkmatter side-effect verbs and expression functions
    6. the single flow-control key, last
- The root-level dictionary form survives; under desugaring it costs nothing extra. It **never carries `when`**: a guarded action requires the list form (a `- when:` item). This is today's rule — the lifecycle-concern key set the parser reads at a stack root excludes `when` — and it is what lets the `loop:` block read its loose verb keys the same way ([Loop Lifecycle](#loop-lifecycle)).
- Audio (`say`/`speak`, `effect`) is **published** to Playa in stack order and never blocks on playback. Playa preserves publication order and serializes playback. See [Background Audio](#background-audio).
- An event is **complete** when its non-audio work has finished and its audio has been published — never "when the audio has played".
- Audio drains across flow-control transitions: when `retry`, `resume`, or `proxy` fires, audio already published for the event keeps playing in queue order and the next flow state's audio queues behind it. There is no cancel API and no job-ID bookkeeping.
- The first flow-control directive that fires **ends the entire event**, wherever in the stack it sits — at the root, inside a `then` body, or inside a nested `else`. Nothing after it in any enclosing list runs.

### Flow-control placement

Because a directive ends the whole event, actions written after an unconditional directive can never run. Claudine treats that as an authoring error rather than silently truncating:

- The rule is **list-local**. Within any one list — a stack root, a `then` body, or an `else` body — an item that unconditionally fires a directive must be the **last** item in that list. Violations are a parse error, `LifecycleActionOrder` (today this error covers ordering within a single item; its meaning is widened to ordering within a list).
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

This grammar can also be used at the root of a stack. That means that the following is valid:

```yaml
start:
    message: "hi"
    say: "nice work"
    shell: "doit"
```

Here the event is a single bundle: after desugaring it runs `message`, then `say`, then `shell`, and the event is complete when `shell` has finished and the audio has been published. A root-level dictionary follows the same rules as a bundled item: each verb at most once, no long form, at most one flow-control key (which runs last), `no_error` permitted, `then`/`else` not permitted. It differs from a bundled item in one way: it **never carries `when`**. There is no guard slot on a root dictionary; the moment an event needs a condition, it is written as a list with a `- when:` item. The key set the parser reads at a stack root already excludes `when` today, so this is a rule the grammar keeps rather than one it adds.

### Long Form

This is not a real change in behavior, the current implementation already supports the idea of a long form action too. The basic idea is that our shorthand syntax of: `{command}: {param}` works very well for most cases because almost all of the actions really only _require_ a single parameter. However, many actions offer optional parameters that give the caller greater control over what the action does.

`no_error` is a parameter of **any** long-form action map, for example `- shell: { command: "post-summary.sh", no_error: true }`. That, and the dictionary-item sibling described above, are its **only two homes**; it is not accepted beside `then`/`else` on a `when` item. The one verb that has neither home is `set`: it has no long form, and a failed `set` is an expression-evaluation error, which `no_error` never suppresses, so `no_error` on `set` is rejected rather than silently ignored.

A good example of this is that every flow-control directive that moves to another document — one that re-reads or recomposes a document — provides an optional `with` parameter that allows the Frontmatter state of the next flow state to be prepared. This is the same `with` that `proxy` already accepts: an overlay applied to the target of the transition, for that transition only, never written to disk and never merged back into the caller. `retry` and the planned `prep` take it too. `resume` does **not**: it sends a follow-up message into the live provider session and continues, so no document is re-read and there is no target to overlay; state that a resumed attempt's later events should see is what `set` is for. (`with` may return on `resume` if it ever gains a defined target.) `break`, `stop`, `skip`, `error`, and `defer` do not take it either, because none of them has a next flow state to prepare.

- the default behavior for flow-state transitions is to move the current state exactly to the new flow-state (which might be the same prompt, a different one, or a sequence)
- the default behavior is good for a lot of flow-state transitions but it is very common that a caller will want to mutate state slightly for the next flow state.
- an example of this is when an agent hits an error of some sort on a prompt and you want to **retry** the action but you want the prompt to know that it's not the first time this has been tried and what the error was last time it happened:

```yaml
failure:
    - retry:
        with:
            reason: "{{ err.msg }}"
        max_attempts: 3
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

### Action Inventory

`action.yaml` enumerates every verb and directive with its short and long form (in the interim schema shape the generated entries are admitted by a catch-all rather than typed one by one; see [Claudine Schema Structure](#claudine-schema-structure)). The inventory below **freezes the source inventory as v1**: it was taken from the parser (`actions.rs`, `action_shape.rs`, `parse.rs`, `signatures.rs` under `claudine/lib/src/composition/lifecycle/`) and from the flow-control topic pages, and it is the one list the schema, the parser, and the docs must agree on. `no_error` applies to every row except `set`.

| Verb | Category | Short-form value | Long-form parameters | Notes |
|---|---|---|---|---|
| `stop` | control | none | — | any event |
| `skip` | control | none | — | `initialize` only |
| `error` | control | `reason` string (opt) | `reason` | |
| `proxy` | control | `target` file-ref (req) | `target` req; `with` mapping | |
| `retry` | control | `max_attempts` number (opt) | `max_attempts`; `delay` duration; `backoff` `fixed`\|`exponential`; `with` mapping | `with` new |
| `resume` | control | `message` string (req) | `message` req; `max_attempts` | needs a live session; no `with` (rule 5) |
| `defer` | control | `delay` duration (req) | `delay` req; `reason` | **planned** |
| `break` | control | `reason` string (opt) | `reason`; `code` int 0..255 | **planned** (`2026-09-27-sequence-improvements`) |
| `prep` | control | `target` file-ref (req) | `target` req; `with` mapping | **planned** (`2026-09-27-sequence-improvements`) |
| `say` / `speak` | communication | text (req) | `text` req | `speak` is an alias |
| `effect` | communication | sound-effect name (req) | `text` req | validated against the effect catalog (`LifecycleUnknownEffect`) |
| `message`, `notify`, `stderr`, `info`, `warn`, `success`, `stdout` | communication | text (req) | `text` req | |
| `shell` | shell | `command` string (req) | `command` req; `on_error` | forbidden in `initialize`; `on_error` is a message string emitted when the command fails, not an ignore switch, so it does not overlap `no_error` |
| `set` | runtime set | mapping only | (none; no long form) | no `no_error` |
| `set_frontmatter` | side-effect | `[file, prop, value]` | `file`, `prop`, `value` | generated |
| `merge_frontmatter` | side-effect | `[file, obj]` | `file`, `obj` | generated |
| `delete_frontmatter` | side-effect | `[file, prop]` | `file`, `prop` | generated |
| `increment_frontmatter` / `decrement_frontmatter` | side-effect | `[file, prop]` | `file`, `prop` | generated |
| `append_frontmatter` / `prepend_frontmatter` | side-effect | `[file, prop, value]` | `file`, `prop`, `value` | generated |
| `ensure_file` | side-effect | `file` or `[file, content]` | `file` req; `content` | generated |
| `ensure_dir` | side-effect | `dir` | `dir` | generated |
| `append_line` | side-effect | `[file, text]` | `file`, `text` | generated |
| `append_jsonl` | side-effect | `[file, obj]` | `file`, `obj` | generated |
| `http_post` | side-effect | `[url, body]` | `url`, `body` | generated |
| 110 expression functions (`file_exists`, `frontmatter`, `length`, `has_command`, ...) | expression-function | scalar, or array zipped to the signature | the catalog's parameter names | generated; the count is whatever the catalog holds at generation time; variadic `and`/`or` are positional-only |

Rules:

1. **Every verb and directive has a long form** `verb: { <params>, no_error?: bool }`, with two exceptions: `set` (rule 4) and the variadic expression functions (rule 2). The short form is the verb's single positional payload: a scalar for a one-parameter verb; an array zipped to the signature for a multi-argument side-effect or expression function; and a bare name, `null`, or `[]` for a zero-argument verb (`- stop`, `- stop: null`). The bare-name spelling is **parser-only**: a schema array item is one type expression, and Darkmatter rejects an array over a union-typed named type ([Expressiveness limits](#expressiveness-limits)), so a bare string cannot sit beside object items in `lifecycle.yaml` until that lands. `- stop: null` is the schema-typable spelling; the parser accepts both.
2. **Side-effect and expression-function entries are generated**, not hand-maintained. The sources are `darkmatter/lib/src/effects/catalog.rs` (12 side-effect verbs, 13 signatures) and `darkmatter/docs/schemas/expression-functions.yaml` (110 functions at the time of writing; the count is whatever the catalog holds at generation time); long-form parameter names are the catalog's parameter names. Variadic functions (`and`, `or`) are positional-only and have no long form. The generator is `spike/gen-action-schema.py`; in the interim schema shape it emits these entries as a single catch-all key rather than typed one by one, and it is retained so full typing can be switched on later ([Claudine Schema Structure](#claudine-schema-structure)).
3. **`break` and `prep` enter as planned entries**, carrying the parameter tables from `2026-09-27-sequence-improvements`. `defer` stays enumerated and planned; the runtime returns `LifecycleDeferNotImplemented` when it is used.
4. **`set` has no long form and no `no_error`.** It stays mapping-only, `set: { key: value }`. A failed `set` is an expression-evaluation error, which `no_error` never suppresses, so the parameter would be meaningless there and is rejected.
5. **`with` is accepted on `retry`, `proxy`, and `prep`** and rejected everywhere else, `resume` included. Its semantics are `proxy`'s overlay in every case: applied to the target of the transition — for `retry` that target is the fresh re-read of the same document — for that transition only, never persisted, never merged back. This is new behavior for `retry` and `prep`; today `LifecycleProxyOnlyParameter` rejects `with` anywhere but `proxy`. That name, and the `LifecycleProxyWith*` error family, become misnomers once `with` is shared by three directives, so they are **renamed at implementation**. `resume` takes no overlay because it sends a follow-up message into the live provider session rather than re-reading a document, so there is nothing to overlay ([Long Form](#long-form)). `break`, `stop`, `skip`, `error`, and `defer` take no overlay.
6. **The communication long form has one payload alias, `text`.** The `message` and `sound` aliases are dropped. The `route` parameter is dropped from the grammar and the schema: it was parsed and validated but never read by the executor (`executor.rs:1323-1343`). It may return when messaging routes are wired.
7. **The count parameter on `retry` and `resume` is `max_attempts`**, the existing name. (An earlier draft of this spec spelled it `max`; that was a typo.)
8. **Confirmed as they are today:** `shell` takes `command` (required) and `on_error`, with no `cwd`, `env`, or `timeout` in v1; `error` takes `reason` only, no `code`; `say`/`speak` take no `voice` or `provider`, because TTS configuration stays in `claudine.toml`; `skip` remains `initialize`-only.
9. The placeholder `claudine/schemas/partials/lifecycle.yaml` is replaced by the `action.yaml` and `lifecycle.yaml` type libraries, or reduced to a reference to `action.yaml`; implementation decides which.

Two long forms side by side, both legal at the root of a list:

```yaml
failure:
    - say: { text: "the run failed", no_error: true }
    - retry:
        max_attempts: 3
        backoff: exponential
        with:
            reason: "{{ err.msg }}"
```

Two doc-drift items are fixed in the [Migration Sweep](#migration-sweep) because the inventory exposed them: the `break` table in `claudine/docs/topics/flow-control/flow-control.md` omits `code`, and `defer`'s `LifecycleDeferNotImplemented` error text still says "requeue". The `with` rules in `flow-control-reference.md` (lines 99–110 today) are rewritten to rule 5.

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
- A `then` or `else` body accepts exactly what a stack accepts — a list, or a dictionary bundle — and does so recursively.
- Because of that, a body written as a dictionary bundle may carry `no_error`, applying to every verb in that body, exactly as a root-level dictionary may. `no_error` remains rejected as a sibling of `then`/`else` on the `when` item itself.
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

Currently conditional blocks can not nest but with this feature we will start to allow nesting. The limit is **five** conditional levels; a sixth is a parse error (`LifecycleStackInvalidShape`). Five is generous — the most common requirement is a second level. The limit is the **parser's**, and only the parser rejects a sixth level. `SimplifiedSchema`, Darkmatter's schema language, has no recursion, so `lifecycle.yaml` unrolls the `when`/`then`/`else` item shape by hand, and because every unrolled level clones the whole action table, it unrolls only **three** fully typed levels: the `then`/`else` bodies that would hold a fourth and fifth level are typed as a loose list-or-map and accepted without deep checking. The editor therefore never falsely rejects a legal document, and never catches a sixth level either; the spike measured why (see [Schema spike (run 2026-09-28)](#schema-spike-run-2026-09-28)). The divergence closes when Darkmatter gains recursive named types ([Package Ownership and Sequencing](#package-ownership-and-sequencing)).

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
- Darkmatter discovers schema roots by an **ancestor walk**: starting from the document's folder it looks for a `schemas/` directory in each parent up to the repository root (the editor workspace root in DMLS). Roots are ordered nearest-first; when two roots hold a file of the same name the nearest one wins and the farther one is reported as shadowed. `md schema triggers` lists every discovered root and trigger and which match arm fired, which is how to check what a document will pick up. The spike confirmed this from `spike/fixtures/` with no `$schema:` pointer: it found `spike/schemas/`, then `claudine/schemas/`, then the repository root, and reported `claudine/schemas/claudine.yaml` as shadowed by the spike's copy.
- Only files of `kind: trigger-schema` **auto-apply**. A trigger-schema is an envelope with a `match:` block and a `$schema` payload; when the match conditions hold for a document, the payload is applied to it. Match conditions are frontmatter-only: `property: <type-expr>` tests, the combinators `all`/`any`/`none`/`min-match`, and `$path` globs. Body-content matching is rejected. DMLS applies matched triggers and reports diagnostics against them.
- Plain `kind: schema` files are **import libraries**. The discovery scan silently ignores them; they reach a document only when named by `$schema` or imported by a trigger's payload.
- The code spells the kind `trigger-schema`; Darkmatter's docs spell it `schema-trigger`, which the parser does not accept. Claudine follows the **code spelling**, confirmed by the spike, until Darkmatter renames one or the other.

### Claudine Schema Structure

Six files, three of them type libraries and three of them triggers whose payloads import from the libraries:

| file path             | kind             | description                                                                                                                                                                                                                                                                             |
|-----------------------|------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `action.yaml`         | `schema`         | types-only library: the action enumeration (every verb and directive, planned ones included), each with its short and long form shapes, per the [Action Inventory](#action-inventory). The 21 hand-authored verbs (nine control directives, ten communication verbs, `shell`, `set`) are typed fully, with literal keys so a typo is diagnosed by name. In the **interim shape** the 12 side-effect verbs and 110 expression functions are admitted by a single-key catch-all (`<string>: any` under `max-keys(1)`) — no editor completion or typo check for them; Claudine's parser validates them at run time. Their entries are **generated** from the two Darkmatter catalogs (`darkmatter/lib/src/effects/catalog.rs`, `darkmatter/docs/schemas/expression-functions.yaml`) by `spike/gen-action-schema.py`, retained so full typing can be switched on once Darkmatter has recursive named types |
| `lifecycle.yaml`      | `schema`         | types-only library built on `action.yaml`: the full shape of a stack — a list of items or a dictionary bundle. Each list item is **one merged mapping** (the verb keys plus `when`, `then`, `else`, and `no_error`, at least one key present) rather than a union of named item types, because Darkmatter cannot type an array over a union-typed name; `then`/`else` are each typed as the property-level union of the next level's item list and the bundle; the parser keeps the exclusivity rules the mapping cannot. Nesting is unrolled to **three** fully typed levels, and the bodies below that are a loose list-or-map accepted without deep checking. The same stack type is what a `loop:` block's `gate:` and a task's `setup:`/`teardown:` carry |
| `linking.yaml`        | `schema`         | types-only library: the eleven optional keys that describe how a prompt is linked into a provider as a skill, command, or agent (`name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns`); imported by the `claudine.yaml` payload so a linked prompt raises no DMLS unknown-key warning |
| `claudine.yaml`       | `trigger-schema` | matches on `any:` of the six event keys (`initialize`, `start`, `blocked`, `success`, `failure`, `finalize`) or `agent`/`model`, each arm spelled `<key>: any(required)`; payload is the catalog of Claudine frontmatter properties below, with the linking keys imported from `linking.yaml`                                        |
| `inline-compose.yaml` | `trigger-schema` | matches `prompt: string(required)` with `none: { sequence: any }`; payload is the shape of an inline-compose document                                                                                                                                                                    |
| `sequence.yaml`       | `trigger-schema` | matches `sequence: any`; payload is the shape of a sequence document, written to the shape `2026-09-27-sequence-improvements` defines (`when:` on a step or task, `loop:` on a group and at the root, `loop` and `seq` reserved) so that spec's examples validate the day the schema lands; every `loop:` block types `gate:`, and every task types `setup:`/`teardown:`, from `lifecycle.yaml` |

The `claudine.yaml` payload describes the frontmatter properties common to every Claudine prompt, taken from `claudine/docs/topics/frontmatter-properties.md`:

- composition core: `prompt`, `agent`, `model`, `interactive`, `$schema`, `last_updated`, `hash`, `fail_fast`, `sequence`, `loop`, `title`, `description`
- the six lifecycle events, typed from `lifecycle.yaml`
- timeouts: `timeout`, `step_timeout`, `timeout_warn`, `step_timeout_warn`
- run controls: `exit_expressions`, `guard_settings`, `operation`, `yolo`, `verbosity`, `mode`
- generated sequence/loop overlays: `state`, `previous_state`, `next_state`, `is_first`, `is_last`, `step`, `total_steps`; the `_loop_*` values become `state.loop.*`
- the linking keys, typed from `linking.yaml`

The linking-only keys (`name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns`) describe how a prompt is linked into a provider as a skill, command, or agent rather than how it composes. They therefore live in their own type library, `linking.yaml`, rather than in the composition catalog itself, and the `claudine.yaml` payload imports that library. The import is what keeps the editor quiet: DMLS reports an unknown frontmatter key as a warning by default (an error only in strict mode), and without the import every linked prompt would carry eleven of them.

### Expressiveness limits

`SimplifiedSchema` can express what the grammar looks like but not everything the grammar requires:

- **Can:** unions (`anyOf`) of inline objects as array items; named types via `Name@this` and imports; pattern keys (`<string>`) with `min-keys`/`max-keys`, which is how the generated-verb catch-all is spelled; `min-keys`/`max-keys` on a literal object of 143 keys, both as `$constraints` and as a postfix on an import (`bundle(max-keys(1))@./action.yaml`).
- **Cannot:** recursion (schemas are a DAG, so nesting is unrolled per level); an ordering constraint such as "the flow-control item must be last" (arrays support only `min`/`max`/`unique`, and the planned tuple form puts the spread last); mutual exclusion or at-most-one-of over keys.
- **Cannot** (established by the spike): an array whose item is a union-typed named type — `item[]@this` where `item` is `bundle | conditional | string` is rejected, which is why a list item is one merged mapping and why the bare `- stop` item is parser-only; a `kind: schema` library carrying a `description` or exporting a `$schema` of its own — a library holds only `kind` and `types`, so it is never validated as a whole file and the event keys are wired by the trigger payload; `required` on a reused named type — the resolver strips it, so `required` is written directly on the property inside the mapping type; a trigger match arm without a presence-requiring condition — `start: any` is rejected as a vacuous arm and `start: any(required)` is the spelling; a shared definition — there is no `$defs`/`$ref`, so every named type is substituted structurally at each reference, and an unrolled schema clones the whole action table once per reference, which is the cost driver the spike measured.

Hence the division of labor: the schemas describe shapes, and Claudine's parser remains the **sole enforcer** of placement (directive last, list-local), cardinality (one flow-control key per bundle, each verb at most once), exclusion (`then`/`else` not beside sibling verbs; no `when` on a root-level dictionary; in a `loop:` block, `gate:` never beside loose verb keys), and the nesting limit beyond the schema's three typed levels. Darkmatter gains no lifecycle machinery for this (see [Package Ownership and Sequencing](#package-ownership-and-sequencing)).

### Schema spike (run 2026-09-28)

The spike ran on 2026-09-28, ahead of planning. Its full record is [`spike-results.md`](spike-results.md); the schemas, fixtures, generator, and harness it used are under `spike/`, and [`spike/README.md`](spike/README.md) says how to reproduce it. It generated `action.yaml` and `lifecycle.yaml` at depth 5 with every verb typed, wrote one trigger, and validated nine fixtures (stacks at depth 1, 3, and 5; a depth-6 rejection; a root-level dictionary; a bundle as a `then` body; a `loop:` `gate:`; a typo'd verb inside `then`; and the long forms) through `md schema triggers`, `md schema validate`, and an in-process harness standing in for DMLS. The three traps it was asked to prove out all held: local references need `@this`; the resolver strips `required` from a reused named type; `trigger-schema` is the accepted envelope spelling.

The grammar fits: every fixture validated, depth 6 was rejected with the exact path, and discovery worked with no `$schema:` pointer. The cost did not fit:

| Configuration | Resolved JSON | Resolve | `md schema validate` | Inlined verb tables |
|---|---|---|---|---|
| depth 1, full table | 1.9 MB | 109 ms | 234 ms | 42 |
| depth 3, full table | 9.5 MB | 657 ms | 1.1 s | 210 |
| depth 5, full table | 39.9 MB | 2.7 s | 5.9 s (3.8–4.1 s via trigger) | 882 |
| depth 5, generated verbs collapsed | 5.7 MB | 403 ms | 1.0 s | 882 |
| depth 3, generated verbs collapsed | 1.4 MB | 82 ms | 266 ms | 210 |

Wall-clock figures were taken on a host under a load average of 22–35 and are upper bounds. The cost is entirely duplication: named types are substituted structurally — there is no `$defs`/`$ref` — so each reference clones the whole 143-key action table, and an unrolled stack inlines it 2^(N+2) − 2 times per consumer across seven consumers (the six events and `gate:`). DMLS caches its assembled schema keyed on the full document text, so every keystroke re-resolves, and no cache helps the first load.

**The resulting shape.** The parser and runtime keep the nesting limit of five. `lifecycle.yaml` unrolls three fully typed levels and types the bodies below them as a loose list-or-map, so the schema never falsely rejects a document and does not reject depth 6; only the parser does. `action.yaml` types the 21 hand-authored verbs fully, with short and long forms, and admits the generated side-effect verbs and expression functions through a single-key catch-all, with the generator retained so `2026-09-28-recursive-schema-types` can switch full typing on. This is the cheapest band measured — about 1.4 MB resolved, 82 ms to resolve, 266 ms per CLI validate — and it was chosen over the alternatives because DMLS re-resolves on every validation, so 0.08 s against 0.4 s is felt, and because it loosens editor validation only at the rare fourth and fifth levels while keeping the typo diagnostic for every hand verb. Editor experience was weighted over one shared number. The target state that removes the unroll and the catch-all, and the Darkmatter defects the spike filed, are recorded under [Package Ownership and Sequencing](#package-ownership-and-sequencing).

### Location

All six files live under `claudine/schemas/`, which the ancestor walk discovers for any document under `claudine/`. A document elsewhere reaches the schema in one of three ways: a `schemas/` directory at the repository root that contains or links the files; a bare-name pointer `$schema: claudine.yaml` resolved against the schema roots; or a `.dmls.toml` `[schema.extensions]` entry naming `claudine/schemas/`.

## Background Audio

Today we have both TTS and sound effects which are played through the host's audio system, via Playa, Claudine's audio playback layer.

The awkwardness in the ordering API is not blocking latency. `say`/`speak` and `effect` already block only on **publication** to Playa's per-user queue (milliseconds; on a TTS cache miss a sequence slot is reserved and a separate preparation process is spawned), and Playa preserves publication order. The time until the first utterance is _audible_ is a separate matter, handled by [Pre-compilation of TTS assets](#pre-compilation-of-tts-assets) below. The ergonomic problem here is only the ordering API:

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

### Pre-compilation of TTS assets

Independent of the ordering change is warming the TTS cache before an event fires: evaluate every `say`/`speak` a document defines and, for phrases whose text is static or interpolates only early-binding state (`ctx`, document globals, frontmatter), produce the audio ahead of time so that _if_ the phrase is needed there is almost no latency.

**The problem, as framed by the owner.** The existing cache does not solve it. A warm cache adds nothing for `initialize` and little for `start` or `blocked`, because a document prepares and fires those within moments of each other; the value is real for the later events (`success`, `failure`, `finalize`, the `loop:` gate), whose phrases are known long before they are needed, and across a sequence the value extends to every event of every step. A TTS provider that consults its cache only for specific voices is a bug, not a feature. What is needed is one uniform way to tell every provider to cache a phrase: providers that do not cache ignore the request, and at play time the providers that did cache detect the hit and benefit.

**What exists today** (established from source):

- biscuit-speaks, the TTS library Claudine plays through via Playa, already has a shared file cache (`biscuit-speaks/lib/src/audio_cache.rs`). The key is `provider:voice_id:text:format[:speed]`, hashed with `biscuit_hash::xx_hash`; entries are written atomically to `{OS temp}/biscuit-speaks-{hash}.{ext}`; there is no eviction and no TTL.
- Providers that produce a file and use that cache: Kokoro, gTTS, Echogarden, ElevenLabs, and macOS `say`, which runs `say -o <tmp>.wav` and copies the result into the shared cache (`say.rs`). The earlier question of whether `say` produces an intermediate file is settled: it does.
- Providers that produce no file: eSpeak (`espeak.rs`) and Windows SAPI (`sapi.rs`). Both are excluded from `requires_preparation`/`cached_job` in `detached.rs` and synthesize with near-zero latency, so there is nothing to pre-compute.
- The cache-miss path: `Speak::play_detached` (`speak.rs`) calls `reserve_preparation` (`detached.rs`), which reserves a Playa queue slot and spawns a detached preparation worker; Playa polls a `Preparing` head slot every 25 ms for up to 10 minutes. `Speak::prepare()` exists but is a stub (`speak.rs:164-167`).
- **Defect.** The cache is consulted only when the voice — and for `say`, the rate — is explicit: `say.rs:447-460` returns `None` unless `resolve_voice` and `resolve_rate` are both `Some`, and ElevenLabs' `cached_detached_job` returns `None` without a `requested_voice` (`elevenlabs.rs:843-848`). Under default settings the cache is never hit for these providers.
- Claudine resolves `say` text at event time (`executor.rs:1000-1052`). A late-binding classifier already exists — `LATE_BINDING_ROOTS = ["err", "timing", "current", "current_env"]` in `claudine/lib/src/composition/reserved.rs`, used by `validate.rs` — and so does prepare-time resolution of early-bound spans (`resolve_string_value`, `executor.rs:926-943`).
- Audible first-utterance latency today: roughly 0.2–0.8 s for `say`, 0.5–3 s for Kokoro on CPU, and 1–3 s for gTTS, Echogarden, and cloud providers; later sounds in the stack queue behind it.

**Decision: a uniform `prepare` API in biscuit-speaks, and a full pre-warm pass in Claudine.** This feature therefore includes a **biscuit-speaks change**; the biscuit-speaks README and `docs/` pages are updated with it, as is the corresponding skill.

biscuit-speaks (estimate 1.5–2.5 days, including tests on the four environments the repository proves: macOS, Linux, native Windows, and WSL2):

- A uniform `prepare(text)` entry point on every provider: the `Speak::prepare` stub is implemented, and the provider trait gains a `prepare` method whose default is a no-op. File-producing providers synthesize into the existing cache; eSpeak and SAPI keep the no-op.
- The default voice and rate are resolved once and **snapshotted**, so the cache key computed at prepare time is the key computed at play time.
- The cache-consult defect is fixed: playback always checks the cache first, whether or not a voice was requested explicitly.
- The `prepare` L1 tests need no audio device, no model download, and no network on any of the four environments. The WSL2 nightly leg has no audio device, and L1 has no network anywhere.

Claudine (estimate 1–2 days):

- At document prepare (the `prepare_document` boundary, which runs before `initialize`), walk every `say`/`speak` phrase in every stack **except `initialize`** (it fires before a prepare could complete). Classify each phrase with `LATE_BINDING_ROOTS`; resolve the early-bound ones against the launch frontmatter and context; call `prepare` for each.
- For a **sequence**, run the same pass over every reachable prompt during sequence preflight, so a phrase in step five is warm before step one starts. "Reachable prompts" means the prompt documents in the preflight graph as it is built today.
- The pass is **fire-and-forget**: it never fails the run and reports at most a warning.
- One setting disables it: `tts.prewarm` (default on), a `claudine.toml` key in the existing `[tts]` section. It is never a frontmatter key. The pass is also skipped under `--dry-run`.
- It is **silent in tests**. It must honor the existing `PLAYA_DRY_RUN`/private-spool fixture boundary or an equivalent biscuit-speaks dry-run switch; implementation decides which, the requirement is that no test triggers synthesis.

The walk is total. It visits every `say`/`speak` in every stack — the five later events, a `loop:` block's `gate:`, and a task's `setup:`/`teardown:` — and inside both `then` and `else` bodies, whatever the guard on the branch says. Classification looks at the phrase text alone: a late-bound guard is irrelevant, because the guard decides whether a phrase plays, not what it says. The cost is the "unfired phrases are synthesized anyway" limit below. Two consequences of `with` being frontmatter-only: `proxy … with` cannot switch the TTS provider or voice, because neither is frontmatter (both live in `claudine.toml`); and a `proxy` or `prep` target's phrases are pre-warmed when that document is itself prepared, after its overlay has been applied, never by the caller.

Limits, stated so that nobody expects more:

- A phrase that names a late-binding root (`err`, `timing`, `current`, `current_env`) is never pre-warmed; it is synthesized at event time exactly as today.
- An early-bound phrase whose frontmatter is mutated by `set` before its event resolves to different text than was pre-warmed. That is a graceful cache miss — the correct text is synthesized at event time — never wrong audio.
- Phrases for events that never fire are synthesized anyway. For a cloud provider that is a billed call of a few dozen characters per unfired phrase.
- Cache entries still have no eviction or TTL; this feature does not add cleanup.

## Loop Lifecycle

The `loop:` block mixes two kinds of keys. Its iteration controls — `while`/`until`, `action`, `max`, `fail_fast`, and the rest — are owned by `2026-09-27-sequence-improvements`, which also owns when the condition is checked, what `action` mutates, the three loop outcomes, and `break`. This feature says one thing about `loop:`: its **lifecycle concerns use the same grammar as every other stack**. They fire on every gate pass — the point at which the loop condition is checked, which the docs already call "the loop gate" — including the terminal pass that exits the loop.

A `loop:` block spells those concerns in one of two ways, and an author picks one:

- **Loose verb keys** at the loop root are the compact spelling. Because the block is a dictionary, they are read as a root-level dictionary bundle: the canonical order, each verb at most once, a single flow-control key last, no `stack:` or `action:` aggregator, and no `when`. (`action`/`actions` at the loop root is the sequence spec's iteration-control key; the looping parser consumes it before the lifecycle parser sees the block, so it is never the removed aggregator.)

```yaml
loop:
    while: "iteration < max_iterations"
    action: increment(iteration)
    stderr: "loop gate"
    info: "finished iteration {{ iteration }} of {{ max_iterations }}"
```

- **`gate:`** is the loop's list home. It holds exactly what any stack accepts — an ordered list, or a dictionary bundle — recursively per the conditional grammar, and it is where a loop puts concerns that need an order the canonical one does not give, or a guard, which the dictionary spelling cannot carry.

```yaml
loop:
    while: "iteration < max_iterations"
    action: increment(iteration)
    gate:
        - effect: "tick"
        - when: "iteration == max_iterations"
          then:
              - warn: "iteration cap reached"
        - info: "finished iteration {{ iteration }} of {{ max_iterations }}"
```

Two shapes are typed parse errors (`LifecycleStackInvalidShape`, or a new name chosen at implementation): **mixing loose verb keys with `gate:`** in one `loop:` block, and **`when` at the loop root**, which is neither an iteration-control key nor a verb — a guard belongs in `gate:`. Rejecting root `when` is also what removes the old `when`/`action` collision at the loop root. One uniform rule results: a list is ordered, a dictionary is compact, everywhere.

`gate:` is typed by `lifecycle.yaml` and reaches sequence documents through `sequence.yaml` ([Schemas](#schemas)). Whether a loop's own block can read the `state.loop.*` values the engine seeds is a defect owned by `2026-09-27-sequence-improvements`, not by this feature; this feature only renames those values (AC7).

## Acceptance Criteria

For a reader new to the repository: tests here are tiered. **L1** tests are hermetic — no network, no real terminal, no audio — and run with `just test` inside a package area (`claudine/`, `darkmatter/`, `biscuit-speaks/`). **L2** tests carry the `level2_` name prefix, drive a real terminal, and run with `just test-l2`. `just lint` runs the linters and, in `claudine/`, the `lint-lifecycle-doc-facets` recipe, a grep gate over the lifecycle topic pages. CI proves Linux and macOS on a pull request, adds Windows on a push to `main`, and adds WSL2 nightly. "Green on the CI schedule" below means those normal runs pass; this feature adds no CI cell.

All criteria are L1 unless marked.

**Parser**

- **AC1** Every stack — each of the six events, a `loop:` block's `gate:`, and a task's `setup:`/`teardown:` — accepts a flat action array; a root-level dictionary form is accepted, never carries `when`, and desugars in the canonical order given in [Execution model](#execution-model).
- **AC2** `when`/`then`/`else` nest to depth 5; depth 6 is a typed parse error naming the path. The rejection is the parser's alone: `lifecycle.yaml` types three levels and accepts the bodies below them loosely, so `md schema validate` and DMLS accept a depth-6 document that the parser rejects.
- **AC3** In any stack, an action after an unconditional flow-control item within the same list is the typed list-local unreachable-action error (`LifecycleActionOrder`); the same verb in a sibling `else` list is legal; a `when` item never counts as unconditional.
- **AC4** `no_error` is accepted in both homes (long-form parameter; dictionary-item sibling applying to every action in the item, a dictionary-bundle body included) and rejected beside `then`/`else` and on `set`.
- **AC5** `with` is accepted on `retry` and `proxy` (and `prep` when built) and rejected elsewhere, `resume` included; a `retry` overlay is visible to the fresh re-read and never persisted.
- **AC6** In any stack, the `loop:` block included: `stack:`; `action:` used as the item aggregator (`- action:`) or as a sibling of `when` on a list item (`- when: … / action: …`); `say_first:`; the communication `route` parameter; and any `_loop_*` read raise a typed removed-grammar `CompositionError` (name final at implementation) carrying replacement guidance, before the provider launches, in compose, inline-compose, and sequence. The `_loop_*` scan covers frontmatter expressions and body spans; where a body read is discoverable only at render time, the same typed error is raised there. The CLI-boundary test is modeled on `compose_removed_validation_keys.rs`. `loop.action`/`loop.actions` are the iteration-control keys of `2026-09-27-sequence-improvements`, consumed by the looping parser first, and are exempt; `when` at the loop root is itself rejected ([Loop Lifecycle](#loop-lifecycle)), so no collision with the aggregator check remains.
- **AC7** `reserved.rs` no longer seeds `_loop_*`; the loop engine seeds `state.loop.{count,is_first,is_last,last_output,last_exit_code}` and `looping/expression.rs` resolves them. `loop` is reserved under `state`: authored state that carries a `loop` key is a typed normalization error, and `set` may not target it, so authored state never silently collides with the ambient values. `seq` stays reserved by `2026-09-27-sequence-improvements`.

**Audio**

- **AC8** `say_first` and `audio_phases` are gone; audio is published in stack order; a lifecycle test asserts publication order and that no test plays sound (the existing `PLAYA_DRY_RUN`/private-spool fixture boundary).
- **AC9** biscuit-speaks `prepare` exists on every provider (a no-op for eSpeak and SAPI), default voice and rate are snapshotted, and playback consults the cache under default settings (a regression test for the explicit-voice-only defect). Claudine pre-warms early-bound phrases in every stack except `initialize`, in `then` and `else` bodies alike, at document prepare and across reachable prompts at sequence preflight; `tts.prewarm = false` in `claudine.toml` and `--dry-run` skip it; tests prove no synthesis is triggered under the test fixture boundary on any of the four environments.

**Shipped artifacts**

- **AC10** `claudine/cli/tests/l1/shipped_prompts.rs` gains a walker that runs the lifecycle parser over every `.md` under `prompts/` and every `claudine/docs/research/**/_fleet.md` plus `_TEMPLATE.md`; zero removed-grammar errors.
- **AC11** `shipped_prompt_route_drift.rs` hashes are refreshed after its fixture is re-derived.
- **AC12** Six files exist: `claudine/schemas/{action,lifecycle,linking}.yaml` (`kind: schema`) and `claudine/schemas/{claudine,inline-compose,sequence}.yaml` (`kind: trigger-schema`). `action.yaml` types the 21 hand-authored verbs fully and admits the generated side-effect verbs and expression functions through a single-key catch-all, with the generator retained; `lifecycle.yaml` uses one merged item mapping per level, types three levels and leaves deeper bodies loose, and types `gate:` and `setup:`/`teardown:`; `claudine.yaml` imports `linking.yaml`. The fixture set is the one under `spike/fixtures/` — depth 1, 3, and 5 valid; depth 6 accepted by the schema and rejected by the parser; root dictionary; bundle in `then`; loop gate; wrong verb diagnosed by name; the long forms — with a zero-argument directive spelled `- stop: null` wherever a fixture is schema-checked (Action Inventory rule 1). They pass `md schema validate` and `md schema triggers` with those verdicts and are picked up by DMLS; `darkmatter/docs/schemas/claudine*.yaml` are deleted; the DMLS `nested_span.rs` mirror and its parity test, the `overlay/frontmatter.rs` pointer tests, and `mapping_only_corpus.rs` pass against the new schema.

**Docs and gates**

- **AC13** The lifecycle, flow-control, flow-control-reference, looping, sequences, side-effects, frontmatter-properties, and getting-started pages show only the new grammar; the looping page no longer promises a one-release `_loop_*` alias period (the old names are a hard error per AC6); `just lint-lifecycle-doc-facets` passes; the biscuit-speaks docs describe `prepare`.
- **AC14** `.claude/skills/claudine/SKILL.md` no longer names `say_first` (`timeline.md` is history and is untouched).
- **AC15** `just test`, `just test-l2`, and `just lint` are green in `claudine/`; `just test` and `just lint` are green in `darkmatter/` (DMLS included) and in `biscuit-speaks/`.
- **AC16** A repository-wide grep for `^\s+stack:|say_first|^\s+- action:|_loop_(count|is_first|is_last|last_output|last_exit_code)` over `prompts/`, `claudine/docs/`, `claudine/lib/`, `claudine/cli/`, `darkmatter/docs/schemas/`, `darkmatter/dmls/`, and `.claude/skills/claudine/` returns only `timeline.md` and spec snapshots.

## Migration Sweep

The grammar this feature removes — `stack:`, `action:` as the item aggregator (`- action:`) or as a sibling of `when` on a list item, `say_first`, the communication `route` parameter, and the `_loop_*` reserved names — is spelled in **65 non-Rust files and 62 Rust files**. All of them are rewritten in this feature. Old spellings are rejected with a typed error and a replacement hint (AC6) rather than silently ignored, and the shipped-prompt gate is extended to run the lifecycle parser (AC10) so the executed prompts, which no test covers today, cannot drift back. The inventory below was taken on 2026-09-28 with false positives removed. Whether the sweep lands as one commit or as per-area commits is a planning choice this spec does not make.

**(a) Executed prompt documents — 48 files.** Run by agents; covered by no test until AC10.

- Root `prompts/` (24): `_add/add-context-variables`, `_add/add-expressions`, `_clarify/findings`, `_implement/implement-plan`, `_implement/implement-review`, `_implement/implement-suggestions`, `_pr/diagnose`, `_pr/dirty`, `_pr/fix`, `_pr/open`, `_pr/push`, `_pr/triage`, `_prompt`, `_reviews/cross-platform`, `_reviews/dry`, `_reviews/feature-review`, `clarify`, `commit`, `format`, `implement`, `merge-conflicts`, `plan`, `pr`, `review`. Of these, `_prompt.md` and `_implement/implement-plan.md` also read `_loop_*`.
- `claudine/docs/research/_TEMPLATE.md` and the 22 research-fleet prompts `claudine/docs/research/*/_fleet.md`: acp, agent-cli, agent-errors, agent-logging, agent-models, agent-permissions, hooks, local_runners, mcp, memory, model-config, non-interactive-sessions, plugins, resume, signals, skills, slash-commands, steering, subagents, system-prompt, usage. These are in scope: they are expected to keep working.
- `claudine/docs/getting-started/index.md`.

**(b) Docs, skills, and schemas — 10 files.**

- `claudine/docs/topics/flow-control/lifecycle.md` (24 `stack:`, 24 `action:`, one `say_first`, one `_loop_`), `flow-control.md`, `flow-control-reference.md`, `looping.md`, `sequences.md`, `state-management/side-effects.md`, `frontmatter-properties.md` (the `_loop_*` table).
    - `looping.md` also promises a one-release alias period for the `_loop_*` names. That is drifted doc: the old names are a hard error (AC6), consistent with the sequence spec's rule that old aliases are removed in the same migration. The sentence is removed under AC13.
    - `sequences.md` includes task `setup:`/`teardown:` examples; those are stacks and are rewritten with the rest of the page.
- `.claude/skills/claudine/SKILL.md` (`say_first` at line 10). `timeline.md` in the same skill is history and stays.
- `darkmatter/docs/schemas/claudine-types.yaml` and `darkmatter/docs/schemas/claudine.yaml`, where the old grammar _is_ the schema; both are deleted (see [Package Ownership and Sequencing](#package-ownership-and-sequencing)).

**(c) Markdown fixtures — 3 files.**

- `claudine/cli/tests/fixtures/nested_span_regression/commit.md`.
- `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`, hash-pinned by `shipped_prompt_route_drift.rs`; refresh with `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 just test-cli shipped_prompt_route_drift::` (AC11).
- `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md`.

**(d) Rust — 62 files**, roughly 165 `stack:` and 445 `action:` lines.

- `claudine/lib/src/composition/lifecycle/**` and `claudine/lib/tests` (16 files, 226 tests); `claudine/cli/tests/l1` (31 files); `claudine/cli/tests/level2` (11 files).
- `claudine/lib/src/composition/sequence/task/**`, the sequence task parser and its tests, where `setup:`/`teardown:` are parsed by the same lifecycle parser; added on review, so not counted in the 62 above.
- `_loop_*` reserved names: 38 in non-test lib source (`looping/expression.rs` 19, `looping/engine.rs` 8, `reserved.rs` 5, `looping/types.rs` 5, `reporting/queries/common.rs` 1); 21 in lib tests; 1 in CLI source (`compose/loop_run.rs`); 5 in CLI tests; 7 in Darkmatter (the `dasherized_identifier_*` tests and `compose/expression/parser.rs`).
- **Cross-package coupling.** `darkmatter/dmls/src/diagnostics/nested_span.rs` carries a static mirror of the lifecycle layout kept in step with `claudine-types.yaml` by a parity test; `darkmatter/dmls/src/overlay/frontmatter.rs` tests assert `initialize.stack[0].when` pointers; `darkmatter/dmls/tests/l1/mapping_only_corpus.rs` runs the Claudine schema extension over a corpus. DMLS moves with the grammar. These are Darkmatter-side test and mirror edits required by the sweep, not new lifecycle machinery (AC12).
- Repository file reads in tests are spelled per the `rust-testing` skill (`workspace_root().join(...)`) so that editing the files they read schedules the narrowed CI cell.

**(e) Spec snapshots — 2 of 42 files.** Only `2026-09-27-sequence-improvements` (`spec.md`, `suggestions.md`) and `2026-09-22-lifecycle-events` (`spec.md`, `against-statement.md`) are rewritten, because both are unbuilt and depend on this grammar. The other 40 are snapshots and stay as written.

**Not swept:** the 40 other spec and review snapshots; `.claude/skills/claudine/timeline.md`; `claudine/docs/research/hooks/*.md`, whose tables describe provider hooks, not lifecycle grammar.

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
2. **Confirm `break` and `prep` are in `action.yaml`** as planned entries with
   the short and long forms recorded in the [Action Inventory](#action-inventory),
   which already enumerates `defer` as planned. Adding them is part of the
   schema deliverable (Action Inventory rule 3, AC12), not a hand-off task;
   this item only checks that they are present and match. The inventory is
   the source; do not restate the parameters from the sequence spec.
3. **Confirm the placement rules here cover the new directives.** The rule is
   list-local: within any one list, an item that unconditionally fires `break`
   or `prep` must be last, exactly as for any other directive, and a `when`
   item never counts as unconditional. `break` inside a nested conditional is
   legal and ends the innermost enclosing loop. `break` in a task `teardown:`
   follows the same list-local rule, because `setup:` and `teardown:` are
   stacks. The sequence spec's `break` and `prep` sections still say a
   directive "ends the current stack", which describes the old silent
   truncation; restate them against the entire-event rule and the list-local
   error so a reader does not infer truncation.
4. **Keep `with` as the one overlay parameter.** Every directive that re-reads
   or recomposes a document (`retry`, `proxy`, `prep`) takes `with`, with
   `proxy`'s existing semantics: an overlay on the target, for that
   transition only, never persisted, never merged back. There is no separate
   `use`. `resume`, `break`, `stop`, `skip`, `error`, and `defer` take no
   overlay (Action Inventory rule 5); `resume` continues the live provider
   session and has no target to overlay.
5. **Write `sequence.yaml` to the shape the sequence spec defines**, not to
   today's shape: `when:` on a step or task, `loop:` on a group object and at
   a sequence document's root, `gate:` inside every `loop:` typed from
   `lifecycle.yaml`, and `loop` and `seq` as reserved authored-state keys.
   Otherwise DMLS rejects every example in that spec the day the schema
   lands.
6. **Keep the "Loop Lifecycle" section above to one job:** the `loop:`
   block's lifecycle concerns use the same grammar as every other stack,
   spelled either as loose verb keys or under `gate:`, never both in one
   block. Everything about when the loop condition is checked, what its
   `action` mutates on each primitive, the three loop outcomes, `break`, and
   whether the loop's own block can read `state.loop.*` is owned by the
   sequence-improvements spec and is not restated here.
7. **Migrate the `_loop_*` reads** in shipped prompts to `state.loop.*` in the
   same pass that rewrites their `stack:`/`action:` blocks, so those files are
   swept once. The engine side, including reserving `loop` under `state`, is
   AC7; the files are bucket (a) of the [Migration Sweep](#migration-sweep).
   The `in_loop` gates in `feature-review.md`, `implement-suggestions.md`, and
   `implement-plan.md` stay until `when:` on steps exists; only their
   spelling changes here. `seq` stays with the sequence spec.
8. **Record the rewrite** in that spec's frontmatter (`grammar: lifecycle-ergonomics`
   or similar) and in this feature's implementation log, set its
   `depends-on` as satisfied, and mark the `_loop_*` rows of its State table
   and its `_loop_*` migration bullet as done by this feature.

The flow-control and looping topic pages already describe `break`, `prep`, and
uniform loops as planned. Their examples are also in the old grammar and are
rewritten under this feature's normal documentation duty, not deferred to the
sequence work.

## Clarification record

This spec was clarified interactively on 2026-09-27 and 2026-09-28. Twelve rulings (R1–R12) were made by the owner and seven reviewer findings (C1–C7) were closed; each is recorded in the section it governs, not here. The schema scale spike ran on 2026-09-28 and its record is [`spike-results.md`](spike-results.md). No rulings remain.

## Open Questions

None remain. The two rulings this section once held — where the linking-only keys live, and whether a `loop:` block has an ordered list home — are recorded in [Schemas](#schemas) (`linking.yaml`) and [Loop Lifecycle](#loop-lifecycle) (`gate:`).

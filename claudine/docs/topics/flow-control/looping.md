# Composition Looping

The idea of a "loop" is a fundamental [flow control](./flow-control.md) primitive. In Claudine we support _looping_ for Markdown (and YAML) authors with the `loop` frontmatter property. This property serves as both a [lifecycle event](./lifecycle.md) while also acting as a [flow control](./flow-control.md) device.

The `loop` property can be attached to any of the three _executable primitives_ Claudine runs:

- a Markdown prompt **document**
- a **sequence** of steps
- a **group** of tasks

> **Planned.** Today only a document's loop runs. A group or sequence cannot declare one yet, and a looping document placed inside a sequence or group currently runs **one** iteration and reports success. Both are defects against this page. Everything here is written for the intended behavior; the loop block has the same shape and the same rules whichever primitive carries it.

A primitive without a `loop` property runs once. When looping is wanted, the primitive you attach it to makes no difference to the rules, so the examples on this page mostly use a Markdown document.

**The condition is checked at the end of each iteration, not before it.** A loop therefore always runs at least once, and the condition decides whether there is a *next* iteration. See [Iteration semantics](#iteration-semantics) for exactly how many times a given condition runs. Whether the primitive runs *at all* is a separate question, answered by a `when:` condition on the step or task (planned) or by `skip` from a document's `initialize`; see [Leaving a loop early](#leaving-a-loop-early).

For a pipeline of _different_ documents, put them in a [sequence](./sequences.md) and loop the sequence, or a group inside it. A document's own loop keeps working when that document is one step of a sequence or one task of a group; the loops nest.

## Frontmatter shape

A loop is declared as a `loop:` object. Where it lives depends on the primitive:

- a **document** carries it at the root of its frontmatter;
- a **group** carries it on the group object, beside `execution` and `tasks`;
- a **sequence document** carries it at the root of its frontmatter, beside `sequence:`. The presence of `sequence:` is what makes the root block the sequence's loop rather than the body's.

```yaml
---
counter: 0
loop:
  while: "counter < 5"
  action: "increment(counter)"
---
```

```yaml
sequence:
    - name: review-repair
      group:
          execution: serial
          loop:
              until: "frontmatter(latest_review, 'ready') == true"
              max: 5
          tasks:
              - prompt: "@prompts/review.md"
              - prompt: "@prompts/repair.md"
```

Recognized keys, identical on every primitive:

| Key         | Type                    | Required                             | Description                                                                                                                                                   |
|-------------|-------------------------|--------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `while`     | string                  | one of `while` / `until` is required | Boolean expression, evaluated after each iteration. Another iteration runs **while** it is truthy.                                                            |
| `until`     | string                  | one of `while` / `until` is required | Boolean expression, evaluated after each iteration. Another iteration runs **until** it is truthy.                                                            |
| `action`    | string \| object \| array | optional                             | One or more frontmatter mutations, applied after an iteration **only when the loop is going to continue**.                                                    |
| `actions`   | string \| object \| array | optional                             | Alias for `action`. Cannot be combined with `action`.                                                                                                         |
| `max`       | positive integer        | optional                             | Iteration cap for this loop. Defaults to `100` when unset.                                                                                                    |
| `fail_fast` | boolean                 | optional                             | When `true` (default), the loop halts on the first iteration failure. When `false`, the loop continues past failures until the condition or the cap stops it. |

Any other key under `loop:` is rejected with a parse error. Common typos like `max_iterations:` and `failfast:` produce a `did you mean …?` hint:

```text
unknown `loop.max_iterations` key (did you mean `max`?); valid keys are: while, until, action, actions, max, fail_fast
```

## Conditions

Conditions use the **Darkmatter expression language**. The full grammar — supported operators, comparison and truthiness rules, helper functions (`Length`, `Contains`, `HasKey`, `And`, `Or`, `number`, `round`), short-circuit semantics, and ternaries — is documented in [Darkmatter Boolean Conditional Logic](@darkmatter/docs/topics/boolean-conditional-logic.md). Treat that doc as authoritative; this section only summarizes what is loop-specific.

What's available inside a loop's `while:` / `until:` expression:

- **Frontmatter properties** of the document (top-level keys and dotted nested paths like `state.phase`).
- **Ambient loop variables** under `state.loop` — see [Ambient variables](#ambient-variables) below.
- **Environment variables** under `env.NAME`. On Windows the name matches without regard to case, as Windows itself does, so `env.PATH` reads the variable Windows reports as `Path`.
- **Runtime context** under `ctx.*` (e.g. `ctx.current_package_area`, `ctx.dirty_files`). Each iteration is a composition run of its own, so it captures its own `ctx`: Git working state (`ctx.branch`, `ctx.staged_files`, `ctx.dirty_files`, …) is observed as the previous iterations left it, while launch-facing values come from the invocation's launch inputs rather than the wrapper's ambient CWD, so the child-working-directory switch cannot make CWD-derived values drift between iterations or steps. See [Composition — Launch-Anchored Prepared Context](../composition.md#launch-anchored-prepared-context).
- **Literals** — strings (`'review'` / `"review"`), numbers, `true`, `false`, `null`.
- **Comparisons** — `==`, `!=`, `>`, `>=`, `<`, `<=`.
- **Boolean operators** — `&&`, `||`, unary `!`, with `&&` binding tighter than `||`.
- **Helper functions** — `Length(...)`, `Contains(...)`, `HasKey(...)`, `number(...)`, `round(...)`, etc. Function names are case-insensitive.
- **Ternaries** — `cond ? a : b`.

```yaml
loop:
  while: "phase < total_phases"
```

```yaml
loop:
  until: "state.loop.last_exit_code == 0"
```

```yaml
loop:
  while: "Contains(state.loop.last_output, 'NEEDS_RETRY') && retries < 3"
```

Because the condition runs after an iteration, `state.loop.last_output` and `state.loop.last_exit_code` name the iteration that **just finished** when the condition reads them. Both examples above therefore react to the run they follow, with no lag.

`while` and `until` are mutually exclusive. Use one or the other.

## Actions

Actions describe how state changes between iterations. They are applied **after the condition has decided that the loop continues**, so iteration `N+1` sees the post-action state. When the condition ends the loop, the actions are not applied, and the final state is the one the last iteration ran with.

What an action mutates depends on the primitive, but the spelling does not. On a **document** it mutates the document's frontmatter. On a **group** or **sequence** it mutates the runtime `set` layer that every task and step reads through ordinary frontmatter keys, so `increment(n)` means the same thing in all three places and a later step sees the new `n`.

The canonical key is `action:`; `actions:` is accepted as an alias. The two cannot be combined in the same document.

The value of `action:` accepts **three shapes**: a single string, a single object, or a list of strings and/or objects. A single string or object is shorthand for a one-element list — semantically the three shapes are identical when only one action is needed.

The following three examples are identical semantically:

**String Format**
```yaml
loop:
    while: "counter < 5"
    action: "increment(counter)"
```

**Object Format**
```yaml
loop:
    while: "counter < 5"
    action:
        op: increment
        prop: counter
```

**List Format**
```yaml
loop:
    while: "counter < 5"
    action:
        - op: increment
          prop: counter
```

On the surface, the **object** and **list** formats look nearly identical and they _are_ in this example but the main reason that the **list** format is included is so that you can include one _or more_ actions on each iteration of the loop.

> **Note:** 
>
> - it is possible that an _action_ causes an error
> - the main reason this would happen is if the _type_ of the Frontmatter property in the document is not able to be mutated with the operation you've chosen
> - any error which occurs in the execution of an action (e.g., a mutation operation) will return an error immediately and stop execution

The whole action list runs as a single staged transaction; see [Action atomicity](#action-atomicity).

### Action atomicity

Each iteration's actions are **all-or-nothing**:

- All actions stage onto a copy of the iteration's pre-action frontmatter.
- The staged copy is committed only when every action succeeds.
- Under `fail_fast: false`, a partially-failed action list discards its stage and the next iteration restarts from the pre-action state.
- Errors include the failing iteration and 1-based action index, e.g. `InvalidIncrementType at iteration 7, action 2 of 4`.

### When templates inside action values are rendered

Action values can contain `{{ ... }}` templates. These are rendered at **action-apply time** (end of the iteration that just ran), against that iteration's effective state:

- Ambient variables (`state.loop.count`, `state.loop.is_first`, `state.loop.is_last`) reflect the iteration whose actions are being applied.
- `state.loop.last_output` and `state.loop.last_exit_code` reflect what the executor produced for that same iteration.
- Frontmatter values reflect the pre-action state of that iteration (earlier actions in the same list have not been applied yet).

A value that is exactly one template span keeps its evaluated JSON type. Anything that mixes template and literal text renders to a string and stays one — it is **never re-parsed as JSON**, because the inserted values are data (`" {{ _loop_last_output }}"` with output `true` stays the string `" true"`). The rendered value is also data for the next iteration's preparation: a `{{ … }}` or `$( … )` it carries is never evaluated again. Specifically:

| Action value                          | After rendering against `{ count: 3, name: "alice" }` |
|---------------------------------------|--------------------------------------------------------|
| `"{{count}}"`                         | `3` (number)                                           |
| `"{{name}}"`                          | `"alice"` (string)                                     |
| `"iter-{{count}}"`                    | `"iter-3"` (string — text + template = string)         |
| `"{{count}} + {{count}}"`             | `"3 + 3"` (string)                                     |
| `"{{count}}{{count}}"`                | `"33"` (string — never re-parsed as a number)          |
| `{ phase: "{{name}}", n: "{{count}}" }` | `{ phase: "alice", n: 3 }` (object walked recursively)  |

Templates are also rendered inside arrays and objects — every string leaf is processed, non-string scalars pass through.

The rule of thumb: **a value that is purely a single template span preserves its evaluated type; anything mixing template with literal text becomes a string.** This means `set(retries, {{state.loop.count}})` lands as a JSON number you can safely compare arithmetically, while `set(label, "iter-{{state.loop.count}}")` lands as the obvious string.

> **Loop vs lifecycle interpolation.** The loop action renderer and the lifecycle event renderer share the same Darkmatter expression core and the same missing-property semantics (an absent property is `null`), and differ in two ways — loop-contextual error typing, and the loop renderer not recognizing `{{{ … }}}` escapes. See [Composition — Loop vs lifecycle interpolation](../composition.md#loop-vs-lifecycle-interpolation); both engines are held to a [shared conformance matrix](../../../lib/src/composition/interpolation_conformance.rs).


## Mutation Operations

In the prior example we saw the **increment** operation being used but the full set of operations are:

| Op                     | Arity | Behavior                                                                                                                                          |
|------------------------|-------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| `increment(prop)`      | 1     | Adds 1 to `prop`. Missing/null → `1`. Numeric strings parse and store back as numbers. Non-numeric strings raise `InvalidIncrementType`.          |
| `decrement(prop)`      | 1     | Subtracts 1 from `prop`. Missing/null → `-1`.                                                                                                     |
| `set(prop,value)`     | 2     | Sets `prop` to `value`. Cannot target reserved names (`loop`, `state`, `replace`, or anything under `state.loop` / `state.seq`).                          |
| `append(prop,value)`  | 2     | Appends `value` to a string property. Objects/arrays serialize to compact JSON and are appended after a `\n` to build JSONL transcripts.          |
| `prepend(prop,value)` | 2     | Reverse of `append`; the `\n` separator goes between the new content and the existing content.                                                    |
| `merge(prop,value)`   | 2     | Shallow object merge.     |

All operations are designed around _changing the state_ of the document's Frontmatter on each iteration turn. Looping without any state change has limited value so these operations are a key part of the utility of looping.

## Ambient Variables

In addition to the _mutation operations_ above, which change real state, the looping engine injects five read-only **ambient variables** into every iteration's effective state. They refer to the **innermost enclosing loop**, so a task inside a looped group reads the group's iteration, and a document that also loops itself reads its own. They live under `state.loop` so they cannot collide with user frontmatter properties.

> **Naming.** This page uses `state.loop.*` (`state.loop.count`, `state.loop.is_first`, `state.loop.is_last`, `state.loop.last_output`, `state.loop.last_exit_code`). `state` is the root every built-in runtime value lives under: a sequence's step object is `state` itself, unchanged, and the sequence overlay moves to `state.seq.*`. `loop` and `seq` are therefore the two names a step's own state may not use. `doc.loop` is the loop's configuration; `state.loop` is what it produced. The current engine spells these five values with a `_loop_` prefix (`_loop_count`, …); those names remain as aliases for one release after the namespace lands, then go away (planned).


| Variable               | Type    | Description                                                                                                                                                                                                          |
|------------------------|---------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `state.loop.count`          | number  | 1-based iteration counter. `1` on the first iteration.                                                                                                                                                               |
| `state.loop.is_first`       | boolean | `true` on iteration 1, `false` thereafter.                                                                                                                                                                           |
| `state.loop.is_last`        | boolean | `true` when this iteration is the final pass the cap permits (`count == max`), `false` otherwise. It says nothing about whether the condition will end the loop; see the note below.                                        |
| `state.loop.last_output`    | string  | In the body and in `start`: captured stdout from the **previous** iteration (empty on iteration 1). In the condition and in actions: stdout from the iteration that **just finished**.                                |
| `state.loop.last_exit_code` | number  | **Process exit code** of a prompt run (Unix-style: `0` = success, non-zero = failure), with the same timing as `state.loop.last_output`. `0` in iteration 1's body because no prior run exists. **Narrow utility — read the note below before using.** |

These variables can be referred to in interpolation, conditional page blocks, your `while`/`until` expression, your mutation operations, and the `start`, `success`, `failure`, and `finalize` lifecycle events.

**Captured output is data.** `state.loop.last_output` holds the agent's text exactly as it was printed, and it stays that way everywhere it goes: in a condition, in an action value, in a lifecycle message, and in the next iteration's prompt. Agents often write template or shell syntax in their summaries; it is shown, never evaluated or run:

```md
---
loop:
  until: "contains(state.loop.last_output, 'DONE')"
  max: 5
---
Continue from where you left off. Your last report was:

{{ state.loop.last_output }}
```

If iteration 1 ends with `see {{…}} and $(rm -rf x)`, iteration 2's prompt contains exactly that text: no parse error, no shell approval, nothing run. The same holds for every value the loop produces, not only the ambient variables: a frontmatter key an action wrote, a value lifted from the composed document, and, inside a sequence, [`outputs`](sequences.md#the-outputs-array). Only values a person typed (`--set`, `key=value`, interactive answers) are templates; see [CLI overrides interact with looping](#cli-overrides-interact-with-looping).

> **`state.loop.is_last` means "last permitted", not "last".** A loop usually ends because of something the iteration itself produced: the agent's output, an exit code, or a file it wrote that the condition reads through `frontmatter(...)`. Nothing can know that before the iteration runs, so `is_last` does not try. It is `true` only on the `max`-th pass. If you want a prompt to say "this is your final attempt" in a loop that runs a fixed number of times, test the count directly: `{{ state.loop.count == 3 }}`, or set `max: 3` and let `is_last` do it.
>
> **Planned.** The current engine instead *predicts* `is_last` by evaluating the condition before the iteration, which is exact for counter loops and silently wrong for every loop that ends on what the agent did, and which can raise when the condition reads a file the agent has not written yet. The cap-only definition above replaces it.

> The `loop:` block's own notification fields and stack (`loop: { info: "…" }`) read the ambient loop values too. There they describe the iteration that just finished, the same values the condition reads.

For example:

```yaml
loop:
  while: "counter < 5"
  action: "set(stamp, {{state.loop.count}})"
```


### When `state.loop.last_exit_code` is actually useful

This variable carries information in **exactly one configuration**: a retry loop that explicitly opts into `fail_fast: false`. Outside that configuration it is always observed as `0` and is effectively dead.

Why it's so narrow:

- **Default behavior is `fail_fast: true`.** A non-zero exit halts the loop immediately, so iteration `N+1` never runs and never sees the failure.
- **Agentic CLIs almost always exit `0` regardless of task success.** They reserve non-zero for catastrophic failures (timeouts, crashes, auth errors, user interrupts), not for "the agent's answer was wrong."
- **It is *not* an iteration counter.** The expression engine has no arithmetic operators, so `{{state.loop.count - 1}}` is *not* a valid template. To get a zero-based counter, maintain it yourself with a `set` or `increment` action on a separate property.

So the practical scope is: **retry on wrapper-detected failure** (typically a timeout or crash) when you've explicitly told the loop to keep going past such failures.

```yaml
---
loop:
  until: "state.loop.last_exit_code == 0"
  fail_fast: false   # required — without this, a failure halts the loop
  max: 3
  action: "increment(attempts)"
attempts: 0
timeout: 5m
---
Try the thing. The loop will retry up to 3 times if the wrapper kills the
child due to timeout, crash, or interrupt.
```

For "did the agent actually succeed at its task?" — exit code is the wrong tool. Use one of these stronger signals instead:

- **Branch on output content** with `state.loop.last_output`:
  ```yaml
  loop:
    until: "contains(state.loop.last_output, 'DONE')"
  ```

- **Use `inline-compose` and have the agent set a sentinel frontmatter property**, then loop on that property:
  ```yaml
  loop:
    until: "done == true"
  done: false
  prompt: "If you finished the task, set frontmatter `done: true`."
  ```

`set(...)` may not write to any ambient name; doing so raises `InvalidAction`. The ambient names are also reserved against being created via shorthand setters or `--set`.

```yaml
---
loop:
  until: "state.loop.count >= 3"
  action: increment(counter)
counter: 0
---

Iteration {{state.loop.count}} of 3.
Counter so far: {{counter}}.
First time? {{state.loop.is_first}}
```

> **Note:** The first looping engine exposed these as bare `iteration`, `is_first`, `is_last`, `last_output`, and `last_exit_code`, which silently shadowed user frontmatter properties; the `_loop_` prefix stopped that, and moving every built-in value under `state` makes the same guarantee for the sequence overlay as well. Prompts still using the bare names must be renamed; prompts using `_loop_*` keep working through the alias period.

## Iteration semantics

`initialize` fires once, before the first iteration. Each iteration then runs a full composition cycle, and the decision about a next iteration is taken at the **loop gate**, which follows that iteration's `finalize`:

1. **Set `state.loop.is_last`** to whether this is the `max`-th pass.
2. **Run the iteration**: compose the body against the iteration's state, fire `start`, launch the agent, fire `success` or `failure`, then `finalize`.
3. **At the loop gate**, in this order:
    1. run any lifecycle concerns authored inside the `loop:` block;
    2. evaluate the condition against **the state this iteration ran with**, plus `state.loop.last_output` and `state.loop.last_exit_code` from this iteration;
    3. if the condition says stop, the loop ends and the actions are **not** applied;
    4. otherwise apply the actions, producing the state iteration `N+1` runs with.

Two consequences are worth committing to memory.

**The first iteration is unconditional.** A condition that is false from the start still runs the prompt once. Whether a primitive runs at all is decided *before* its loop, by a `when:` condition on the sequence step or group task that names it (planned; a false `when:` records the primitive as skipped and runs no iteration). Inside a document, the same opt-out is a `skip` from `initialize`:

```yaml
initialize:
    stack:
        - when: "length(work) == 0"
          action:
              - info: "nothing to do"
              - skip
```

**The condition is asked of the iteration that just ran, so a counter counts one further than it reads.** With `n: 0` and `action: "increment(n)"`:

| Condition | Iterations | Each body sees |
|---|---|---|
| `while: "n < 0"` | 1 | `n=0` |
| `while: "n < 1"` | 2 | `n=0`, `n=1` |
| `while: "n < 2"` | 3 | `n=0`, `n=1`, `n=2` |
| `until: "n == 2"` | 3 | `n=0`, `n=1`, `n=2` |
| `until: "state.loop.count >= 3"` | 3 | `state.loop.count` of `1`, `2`, `3` |
| `until: "state.loop.count > 3"` | 4 | `state.loop.count` of `1`, `2`, `3`, `4` |

Read a condition as "*did the iteration that just finished satisfy this?*" rather than "*may the next one start?*". `until: "n == 2"` stops after the iteration that ran with `n=2`. For exactly `k` runs, the clearest spelling is `until: "state.loop.count >= k"`.

### Leaving a loop early

- **`break` (planned)** ends the innermost enclosing loop, whichever primitive owns it. The current iteration completes, `finalize` included, and the loop then stops without evaluating its condition or applying its actions. A loop ended by `break` is reported as its own outcome, distinct from "condition met" and "`max` reached". See [Flow Control — `break`](flow-control.md#break).
- An explicit `error` in the `loop:` block's stack fails the run before the condition is evaluated.
- A `proxy` raised by an iteration's `start`, `success`, `failure`, or `finalize` ends the loop and hands off to its target, which enters at its own `initialize` and is not an extra iteration of this loop. No further iteration runs and the `loop:` block does not fire for the abandoned iteration. A proxy from `success` or `failure` skips that iteration's `finalize`, and one from `finalize` does not run it again.
- `retry`, `resume`, and `proxy` authored inside the `loop:` block itself are not supported and fail the run with `LifecycleSetupPhaseRecoveryUnsupported`.

## Iteration cap

Every loop is bounded so a runaway condition cannot hang indefinitely.

- **Default cap:** `100` iterations.
- **Per-loop override:** `loop.max: <N>` on the primitive.
- **Per-run override:** `--max-iterations <N>` on the CLI, or the `CLAUDINE_MAX_ITERATIONS` environment variable.

Precedence is **CLI > environment > frontmatter > built-in default**.

The `max`-th iteration always sees `state.loop.is_last: true`. If the condition still says "continue" after it, the loop exits with `LoopLimitExceeded`, carrying the prompt path and the iteration number that breached the cap, and the run fails. A loop that reaches its cap and whose condition then says "stop" ends normally.

A loop therefore ends in one of three ways, and sequence summaries and exit codes distinguish them (planned): the condition was met, `max` was reached, or a `break` fired.

## Fail-fast semantics

`fail_fast` controls how per-iteration failures propagate:

| Source of failure                                    | `fail_fast: true` (default)                                                 | `fail_fast: false`                                                              |
|------------------------------------------------------|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| Prompt run exits non-zero                            | Loop halts; `final_exit_code` is the failing run's code.                    | Loop continues; the next iteration sees the failure via `state.loop.last_exit_code`. |
| An `error` in `success` that nothing recovers        | Loop halts; the error names the iteration and the `error` reason.           | Loop continues through the gate; the iteration's reason is reported as it ends. |
| The iteration cannot complete (e.g. a refused `proxy`: missing target, cycle, hop limit) | Loop halts with that error.                                  | The error is reported and the next iteration starts; the `loop:` gate does not run for the failed iteration. |
| Action raises an error (e.g. `InvalidIncrementType`) | Loop halts; the action stage for that iteration is discarded.               | Loop continues; the iteration's frontmatter remains in its pre-action state.    |
| Loop condition cannot be parsed/evaluated            | Loop halts unconditionally — this is a structural error, not a runtime one. | Same.                                                                           |

Per-run override: `--fail-fast=true|false` on the CLI, or the `CLAUDINE_FAIL_FAST` environment variable. Precedence is **CLI > environment > frontmatter > built-in default (true)**.

## User interrupt (Ctrl+C)

Pressing **Ctrl+C** at any point during a `compose` / `inline-compose` run — including the pre-launch prep window where the source file is parsed, transclusions are resolved, target/model are selected, and shell preflight runs — halts the run with a friendly notice. The CLI installs a process-scoped `SIGINT` handler at the very top of the subcommand (just after positional argv parsing, before any I/O-heavy prep), so the handler is live throughout. It fires alongside the wrapper's per-iteration handler, so even when an agent CLI exits `0` after being interrupted (most do), the loop stops between iterations rather than starting a new one. When the interrupt is observed during loop execution the loop halts regardless of `fail_fast`. Post-prep checkpoints consult the same flag and bail out with exit code `130` before launching the agent if the user interrupted during prep.

On interrupt the CLI:

- **Immediately** writes an INFO status line directly to stderr from the signal handler — this lands *before* the interrupted agent's dying-breath events are rendered, so it is the first thing the operator sees after the terminal echoes `^C`. The line is column-1 aligned (a leading newline pushes it off the `^C`) and the prompt path is rendered as an OSC8 hyperlink with the visible text resolved relative to the repo root (or CWD when not in a repo).
- **Relabels** the agent's terminal-error block from `Agent Error` to `User Action — User pressed CTRL+C to stop the session` with a yellow border, so operators are not led to believe the agent itself failed.
- Returns exit code `130` (the standard `128 + SIGINT(2)` shell convention).
- Surfaces a `LoopInterrupted` error in the in-process [`LoopExecutionResult`](../../../lib/src/composition/looping/engine.rs) for programmatic callers, but does **not** print a redundant red `Error:` line — the INFO status is the only user-facing announcement.

Implementation notes:

- The signal handler uses `libc::write(2, …)` (async-signal-safe) on a pre-rendered byte buffer; the Rust stdio macros (`eprintln!`, `tracing`) are *not* signal-safe and must not be added to that path.
- A process-scoped `USER_INTERRUPTED` atomic in [`output/mod.rs`](../../../cli/src/output/mod.rs) lets the live semantic sink's error renderer detect the interrupt and remap the label without plumbing flags through every parser.

## CLI overrides interact with looping

CLI shorthand setters and `--set` JSON are applied to the **initial frontmatter** that the loop's first iteration sees. So:

```bash
claudine compose loop_example.md iteration=1 --claude
```

…is equivalent to authoring the document with `iteration: 1` in the frontmatter. Subsequent iterations carry that value through unless an action explicitly overwrites it.

A setter is authored text, so a template in it fills in on every iteration: `--set '{"label":"{{ title }}"}'` renders the current `title` each time. Once an action overwrites that key, the key holds the action's result, which is data.

## Common errors

| Error                                                               | Cause                                                                                                                 |
|---------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `unknown loop.<key> key`                                            | A typo or unsupported key under `loop:`. The error message lists valid keys and offers a suggestion for common typos. |
| `loop.action and loop.actions are aliases; specify only one`        | Both keys were set in the same document. Pick one.                                                                    |
| `loop.while and loop.until are mutually exclusive`                  | Both `while:` and `until:` were set. Pick one.                                                                        |
| `loop must define either while or until`                            | Neither condition key is present.                                                                                     |
| `loop.max must be greater than zero`                                | `max:` is `0` or negative.                                                                                            |
| `InvalidIncrementType at iteration N, action M of K`                | An `increment` / `decrement` target resolved to a non-numeric, non-numeric-string value.                              |
| `InvalidAction at iteration N, action M of K: '<prop>' is reserved` | An action tried to write to `loop`, `state`, `replace`, or anything under `state.loop` / `state.seq`.                         |
| `LoopLimitExceeded`                                                 | The cap was reached and the condition would still continue.                                                           |
| `LoopInterrupted`                                                   | The user pressed Ctrl+C; the loop halted between iterations and exited with code `130`.                               |

## See also

- [Composition](../composition.md) — how `compose` and `inline-compose` flow into the wrapper pipeline.
- [Sequences](./sequences.md) — multi-document pipelines with shared shell approval and per-step provider review.
- [Lifecycle](lifecycle.md) — `start` / `success` / `blocked` / `failure` notifications. These render with the **iteration's** frontmatter, so templates like `{{state.loop.count}}` and any user property mutated by actions reflect each iteration's state.

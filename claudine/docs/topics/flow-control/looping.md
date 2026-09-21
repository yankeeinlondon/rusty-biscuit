# Composition Looping

Claudine compositions support **per-prompt loops** declared in Markdown frontmatter. A loop wraps a single composition document so the same prompt is executed repeatedly until a stopping condition is met, with optional frontmatter mutations between iterations.

**The condition is checked at the end of each iteration, not before it.** A loop therefore always runs at least once, and the condition decides whether there is a *next* iteration. See [Iteration semantics](#iteration-semantics) for exactly how many times a given condition runs.

Looping applies to `claudine compose` and `claudine inline-compose`. For multi-step pipelines composed of _different_ documents, use [`claudine sequence`](./sequences.md) instead.

## Frontmatter shape

A loop is declared as a `loop:` object in the document's frontmatter:

```yaml
---
counter: 0
loop:
  while: "counter < 5"
  action: "increment(counter)"
---
```

Recognized keys:

| Key         | Type                    | Required                             | Description                                                                                                                                                   |
|-------------|-------------------------|--------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `while`     | string                  | one of `while` / `until` is required | Boolean expression, evaluated after each iteration. Another iteration runs **while** it is truthy.                                                            |
| `until`     | string                  | one of `while` / `until` is required | Boolean expression, evaluated after each iteration. Another iteration runs **until** it is truthy.                                                            |
| `action`    | string \| object \| array | optional                             | One or more frontmatter mutations, applied after an iteration **only when the loop is going to continue**.                                                    |
| `actions`   | string \| object \| array | optional                             | Alias for `action`. Cannot be combined with `action`.                                                                                                         |
| `max`       | positive integer        | optional                             | Per-document iteration cap. Defaults to `100` when unset.                                                                                                     |
| `fail_fast` | boolean                 | optional                             | When `true` (default), the loop halts on the first iteration failure. When `false`, the loop continues past failures until the condition or the cap stops it. |

Any other key under `loop:` is rejected with a parse error. Common typos like `max_iterations:` and `failfast:` produce a `did you mean …?` hint:

```text
unknown `loop.max_iterations` key (did you mean `max`?); valid keys are: while, until, action, actions, max, fail_fast
```

## Conditions

Conditions use the **Darkmatter expression language**. The full grammar — supported operators, comparison and truthiness rules, helper functions (`Length`, `Contains`, `HasKey`, `And`, `Or`, `number`, `round`), short-circuit semantics, and ternaries — is documented in [Darkmatter Boolean Conditional Logic](@darkmatter/docs/topics/boolean-conditional-logic.md). Treat that doc as authoritative; this section only summarizes what is loop-specific.

What's available inside a loop's `while:` / `until:` expression:

- **Frontmatter properties** of the document (top-level keys and dotted nested paths like `state.phase`).
- **Ambient loop variables** under the `_loop_` prefix — see [Ambient variables](<#Ambient Variables>) below.
- **Environment variables** under `env.NAME`.
- **Runtime context** under `ctx.*` (e.g. `ctx.current_package_area`). Canonical preparation stores the exact `ComposeContext` derived from the invocation's launch inputs and the active document's source. Loop iterations and sequence steps derive from that request snapshot rather than recapturing the wrapper's ambient CWD, so the child-working-directory switch cannot make CWD-derived values drift between iterations or steps.
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
  until: "_loop_last_exit_code == 0"
```

```yaml
loop:
  while: "Contains(_loop_last_output, 'NEEDS_RETRY') && retries < 3"
```

Because the condition runs after an iteration, `_loop_last_output` and `_loop_last_exit_code` name the iteration that **just finished** when the condition reads them. Both examples above therefore react to the run they follow, with no lag.

`while` and `until` are mutually exclusive. Use one or the other.

## Actions

Actions describe how frontmatter changes between iterations. They are applied **after the condition has decided that the loop continues**, so iteration `N+1` sees the post-action state. When the condition ends the loop, the actions are not applied, and the final state is the one the last iteration ran with.

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

## Mutation Operations

In the prior example we saw the **increment** operation being used but the full set of operations are:

| Op                     | Arity | Behavior                                                                                                                                          |
|------------------------|-------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| `increment(prop)`      | 1     | Adds 1 to `prop`. Missing/null → `1`. Numeric strings parse and store back as numbers. Non-numeric strings raise `InvalidIncrementType`.          |
| `decrement(prop)`      | 1     | Subtracts 1 from `prop`. Missing/null → `-1`.                                                                                                     |
| `set(prop,value)`     | 2     | Sets `prop` to `value`. Cannot target reserved names (`loop`, `replace`, or any `_loop_*` ambient name).                                          |
| `append(prop,value)`  | 2     | Appends `value` to a string property. Objects/arrays serialize to compact JSON and are appended after a `\n` to build JSONL transcripts.          |
| `prepend(prop,value)` | 2     | Reverse of `append`; the `\n` separator goes between the new content and the existing content.                                                    |
| `merge(prop,value)`   | 2     | Shallow object merge.     |

All operations are designed around _changing the state_ of the document's Frontmatter on each iteration turn. Looping without any state change has limited value so these operations are a key part of the utility of looping.

## Ambient Variables

In addition to the aforementioned _mutation operations_ which mutate "real state", there are a set of _ambient variables_ which Claudine will set for you during a looping operation:


| Variable               | Type    | Description                                                                                                                                                                                                          |
|------------------------|---------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `_loop_count`          | number  | 1-based iteration counter. `1` on the first iteration.                                                                                                                                                               |
| `_loop_is_first`       | boolean | `true` on iteration 1, `false` thereafter.                                                                                                                                                                           |
| `_loop_is_last`        | boolean | A **prediction made before the iteration runs**: `true` when the condition already says "stop" against this iteration's state, **or** when this iteration is the `max`-th. See the caveat below.                        |
| `_loop_last_output`    | string  | In the body and in `start`: captured stdout from the **previous** iteration (empty on iteration 1). In the condition and in actions: stdout from the iteration that **just finished**.                                |
| `_loop_last_exit_code` | number  | **Process exit code** of a prompt run (Unix-style: `0` = success, non-zero = failure), with the same timing as `_loop_last_output`. `0` in iteration 1's body because no prior run exists. **Narrow utility — read the note below before using.** |

These variables can be referred to in interpolation, conditional page blocks, your `while`/`until` expression, your mutation operations, and the `start`, `success`, `failure`, and `finalize` lifecycle events.

> **`_loop_is_last` is a prediction, not a fact.** It is computed before the iteration starts, from the state that iteration begins with and the *previous* iteration's output. When the condition depends on something the iteration itself changes — `_loop_last_output`, `_loop_last_exit_code`, or a file the agent writes and the condition reads through `frontmatter(...)` — the iteration that actually turns out to be last will have seen `_loop_is_last: false`. It is exact for conditions over `_loop_count` and over frontmatter that only the loop's own actions change, and it is always `true` on the `max`-th iteration.

> **Known defect.** The `loop:` block's own notification fields and stack (`loop: { info: "…" }`) cannot read the `_loop_*` values; referencing one there fails the run with an "unknown root" error. They can read ordinary frontmatter. Tracked as F8 in the fix `2026-09-20-lifecycle-handoff-gaps`.

For example:

```yaml
loop:
  while: "counter < 5"
  action: "set(stamp, {{_loop_count}})"
```



The whole list runs as a single staged transaction — see [Action atomicity](<looping#Action atomicity>) below.


### Action atomicity

Each iteration's actions are **all-or-nothing**:

- All actions stage onto a copy of the iteration's pre-action frontmatter.
- The staged copy is committed only when every action succeeds.
- Under `fail_fast: false`, a partially-failed action list discards its stage and the next iteration restarts from the pre-action state.
- Errors include the failing iteration and 1-based action index, e.g. `InvalidIncrementType at iteration 7, action 2 of 4`.

### When templates inside action values are rendered

Action values can contain `{{ ... }}` templates. These are rendered at **action-apply time** (end of the iteration that just ran), against that iteration's effective state:

- Ambient variables (`_loop_count`, `_loop_is_first`, `_loop_is_last`) reflect the iteration whose actions are being applied.
- `_loop_last_output` and `_loop_last_exit_code` reflect what the executor produced for that same iteration.
- Frontmatter values reflect the pre-action state of that iteration (earlier actions in the same list have not been applied yet).

After rendering, the result is **re-parsed as JSON** so that numeric, boolean, and `null` template results land as their proper JSON types. Non-JSON results fall back to a string. Specifically:

| Action value                          | After rendering against `{ count: 3, name: "alice" }` |
|---------------------------------------|--------------------------------------------------------|
| `"{{count}}"`                         | `3` (number)                                           |
| `"{{name}}"`                          | `"alice"` (string)                                     |
| `"iter-{{count}}"`                    | `"iter-3"` (string — text + template = string)         |
| `"{{count}} + {{count}}"`             | `"3 + 3"` (string)                                     |
| `{ phase: "{{name}}", n: "{{count}}" }` | `{ phase: "alice", n: 3 }` (object walked recursively)  |

Templates are also rendered inside arrays and objects — every string leaf is processed, non-string scalars pass through.

The rule of thumb: **a value that is purely a single template span preserves its evaluated type; anything mixing template with literal text becomes a string.** This means `set(retries, {{_loop_count}})` lands as a JSON number you can safely compare arithmetically, while `set(label, "iter-{{_loop_count}}")` lands as the obvious string.

> **Loop vs lifecycle interpolation.** The loop action renderer and the lifecycle event renderer share the same Darkmatter expression core but differ in three deliberate ways — the JSON re-parse above (loop only), loop-contextual error typing, and unknown-root leniency (loop) vs strict fail-closed (lifecycle). See [Composition — Loop vs lifecycle interpolation](../composition.md#loop-vs-lifecycle-interpolation); both engines are held to a [shared conformance matrix](../../../lib/src/composition/interpolation_conformance.rs).

## Ambient variables

The looping engine injects five read-only **ambient variables** into every iteration's effective state. They are namespaced under the `_loop_` prefix to avoid colliding with user frontmatter properties:


### When `_loop_last_exit_code` is actually useful

This variable carries information in **exactly one configuration**: a retry loop that explicitly opts into `fail_fast: false`. Outside that configuration it is always observed as `0` and is effectively dead.

Why it's so narrow:

- **Default behavior is `fail_fast: true`.** A non-zero exit halts the loop immediately, so iteration `N+1` never runs and never sees the failure.
- **Agentic CLIs almost always exit `0` regardless of task success.** They reserve non-zero for catastrophic failures (timeouts, crashes, auth errors, user interrupts), not for "the agent's answer was wrong."
- **It is *not* an iteration counter.** The expression engine has no arithmetic operators, so `{{_loop_count - 1}}` is *not* a valid template. To get a zero-based counter, maintain it yourself with a `set` or `increment` action on a separate property.

So the practical scope is: **retry on wrapper-detected failure** (typically a timeout or crash) when you've explicitly told the loop to keep going past such failures.

```yaml
---
loop:
  until: "_loop_last_exit_code == 0"
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

- **Branch on output content** with `_loop_last_output`:
  ```yaml
  loop:
    until: "contains(_loop_last_output, 'DONE')"
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
  until: "_loop_count >= 3"
  action: increment(counter)
counter: 0
---

Iteration {{_loop_count}} of 3.
Counter so far: {{counter}}.
First time? {{_loop_is_first}}
```

> **Note:** Earlier versions of the looping engine exposed these as `iteration`, `is_first`, `is_last`, `last_output`, and `last_exit_code` — bare names that silently shadowed user frontmatter properties. The `_loop_` prefix prevents that shadowing. If you have older prompts using the unprefixed names, rename them before running under the current engine.

## Iteration semantics

`initialize` fires once, before the first iteration. Each iteration then runs a full composition cycle, and the decision about a next iteration is taken at the **loop gate**, which follows that iteration's `finalize`:

1. **Predict `_loop_is_last`** from the state this iteration begins with.
2. **Run the iteration**: compose the body against the iteration's state, fire `start`, launch the agent, fire `success` or `failure`, then `finalize`.
3. **At the loop gate**, in this order:
    1. run any lifecycle concerns authored inside the `loop:` block;
    2. evaluate the condition against **the state this iteration ran with**, plus `_loop_last_output` and `_loop_last_exit_code` from this iteration;
    3. if the condition says stop, the loop ends and the actions are **not** applied;
    4. otherwise apply the actions, producing the state iteration `N+1` runs with.

Two consequences are worth committing to memory.

**The first iteration is unconditional.** A condition that is false from the start still runs the prompt once. To run zero times, opt the whole document out with a `skip` from `initialize`:

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
| `until: "_loop_count >= 3"` | 3 | `_loop_count` of `1`, `2`, `3` |
| `until: "_loop_count > 3"` | 4 | `_loop_count` of `1`, `2`, `3`, `4` |

Read a condition as "*did the iteration that just finished satisfy this?*" rather than "*may the next one start?*". `until: "n == 2"` stops after the iteration that ran with `n=2`. For exactly `k` runs, the clearest spelling is `until: "_loop_count >= k"`.

### Leaving a loop early

- An explicit `error` in the `loop:` block's stack fails the run before the condition is evaluated.
- A `proxy` raised by an iteration's `success`, `failure`, or `finalize` is meant to end the loop and hand off to its target, which is not an extra iteration of this loop. **Known defect:** the handoff is currently not performed and the loop continues; tracked as F2 in the fix `2026-09-20-lifecycle-handoff-gaps`. Until it is fixed, use a lifecycle `retry` for bounded repetition in a document that also has to hand off.
- `retry`, `resume`, and `proxy` authored inside the `loop:` block itself are not supported and fail the run with `LifecycleSetupPhaseRecoveryUnsupported`.

## Iteration cap

Every loop is bounded so a runaway condition cannot hang indefinitely.

- **Default cap:** `100` iterations.
- **Per-document override:** `loop.max: <N>` in frontmatter.
- **Per-run override:** `--max-iterations <N>` on the CLI, or the `CLAUDINE_MAX_ITERATIONS` environment variable.

Precedence is **CLI > environment > frontmatter > built-in default**.

The `max`-th iteration always sees `_loop_is_last: true`. If the condition still says "continue" after it, the loop exits with `LoopLimitExceeded`, carrying the prompt path and the iteration number that breached the cap, and the run fails. A loop that reaches its cap and whose condition then says "stop" ends normally.

## Fail-fast semantics

`fail_fast` controls how per-iteration failures propagate:

| Source of failure                                    | `fail_fast: true` (default)                                                 | `fail_fast: false`                                                              |
|------------------------------------------------------|-----------------------------------------------------------------------------|---------------------------------------------------------------------------------|
| Prompt run exits non-zero                            | Loop halts; `final_exit_code` is the failing run's code.                    | Loop continues; the next iteration sees the failure via `_loop_last_exit_code`. |
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

## Common errors

| Error                                                               | Cause                                                                                                                 |
|---------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------|
| `unknown loop.<key> key`                                            | A typo or unsupported key under `loop:`. The error message lists valid keys and offers a suggestion for common typos. |
| `loop.action and loop.actions are aliases; specify only one`        | Both keys were set in the same document. Pick one.                                                                    |
| `loop.while and loop.until are mutually exclusive`                  | Both `while:` and `until:` were set. Pick one.                                                                        |
| `loop must define either while or until`                            | Neither condition key is present.                                                                                     |
| `loop.max must be greater than zero`                                | `max:` is `0` or negative.                                                                                            |
| `InvalidIncrementType at iteration N, action M of K`                | An `increment` / `decrement` target resolved to a non-numeric, non-numeric-string value.                              |
| `InvalidAction at iteration N, action M of K: '<prop>' is reserved` | An action tried to write to `loop`, `replace`, or any `_loop_*` ambient name.                                         |
| `LoopLimitExceeded`                                                 | The cap was reached and the condition would still continue.                                                           |
| `LoopInterrupted`                                                   | The user pressed Ctrl+C; the loop halted between iterations and exited with code `130`.                               |

## Implementation note

`claudine compose` and `claudine inline-compose` drive loops through `execute_loop_with_lifecycle` in [`looping/engine.rs`](../../../lib/src/composition/looping/engine.rs), and that is the engine this page describes. The same file still holds `execute_loop` and `execute_loop_with_config`, an older engine that checks the condition **before** each iteration and so can run zero times. No command uses it, only library tests call it, and it is scheduled for deletion by the fix `2026-09-20-lifecycle-handoff-gaps` (R7). Do not call it; this note goes away with it.

## See also

- [Composition](../composition.md) — how `compose` and `inline-compose` flow into the wrapper pipeline.
- [Sequences](./sequences.md) — multi-document pipelines with shared shell approval and per-step provider review.
- [Lifecycle](../lifecycle.md) — `start` / `success` / `blocked` / `failure` notifications. These render with the **iteration's** frontmatter, so templates like `{{_loop_count}}` and any user property mutated by actions reflect each iteration's state.

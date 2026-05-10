While a lot of the flow control in Claudine is managed by **sequence**'s in this feature we'll add an atomic
flow control feature to the individual prompt called "looping" and defined by the `loop` Frontmatter property:

- it is a very common requirement to need to _loop over_ a certain prompt file and that is the primary utility that this feature will provide
- the `loop` property must consist of both a "conditional" (to determine when to stop looping) and 99% of the time a "action mutator" that will change the Frontmatter state in some way during the iteration.

## Syntax

The `loop` property accepts both scalar and list forms for `actions`. Each entry in the list may be either a DSL string OR a structured object. Order of execution equals order of appearance.

```yaml
# Single action, DSL-string form
loop:
  while: "counter < 5"
  actions: "increment(counter)"
```

```yaml
# Multiple actions, list of DSL strings (order = list order)
loop:
  while: "counter < 5"
  actions:
    - "increment(counter)"
    - "append(log, '{{iteration}}: {{last_output}}')"
```

```yaml
# Structured-object form
loop:
  until: "done"
  actions:
    - { op: increment, prop: counter }
    - { op: set, prop: stage, value: review }
```

> **Note:**
>
> The plural key `actions:` is authoritative. A scalar value (single DSL string or single object) is shorthand for a one-element list.

## Conditionals

Conditionals are defined by one of the operators (as a frontmatter property under `loop`) defined in the next section.

### Operators

- `while`
    - receives a boolean expression to evaluate and so long as the expression evaluates to `true` it will continue to iterate
- `until`
    - receives a boolean expression to evaluate and so long as the expression evaluates to `false` it will continue to iterate

### Conditional Expressions

The conditional expressions allowed are those defined in Darkmatter in the @darkmatter/docs/topics/darkmatter-expressions.md document.

> **Note:**
>
> Conditions are evaluated against the document's effective state, which includes the (mutated) frontmatter for the upcoming iteration **plus** the [Ambient Variables](#ambient-variables) injected by the looping engine.

## Action Mutators

The action mutator(s) are defined by the `actions` frontmatter property under the `loop` property (see [Syntax](#syntax) for accepted shapes). A looping prompt can have 0:M actions on each loop but these actions are only _applied_ after the completion of a prompt (aka, no action taken when the prompt is first executed). This follows directly from the [pre-check iteration order](#iteration-semantics).

### Action/Mutation Operations

- `increment(prop)`

    Increments the specified property by 1.

    > **Note:**
    >
    > - if the property specified is empty/null/undefined then incrementing will make it **1**
    > - if the property specified is not a numeric (or a string representation of a number) property then this will cause a `InvalidIncrementType` error to be returned; stopped execution immediately.

- `decrement(prop)`

    Decrements the specified property by 1.

    > **Note:**
    >
    > - if the property specified is empty/null/undefined then decrementing will make it **-1**
    > - if the property specified is not a numeric (or a string representation of a number) property then this will cause a `InvalidDecrementType` error to be returned; stopped execution immediately.

- `set(prop, value)`

    Sets a property in the next iteration of the Frontmatter.

    > **Note:**
    >
    > - you can not set the "loop" or "replace" properties and if you do then it will cause a `InvalidAction` error; stopped execution immediately.

- `append(prop, value)`

    Appends a value `value` to the Frontmatter property `prop`.
    - this is intended to be used with a Frontmatter property with string content, but:
        - if the value is numeric or boolean it will be converted to a string equivalent before being appended
        - if the ${value} is an object/dictionary then it will be serialized to JSON (in a compact form with no new lines) and then a "\n" character placed at the front before being appended to the frontmatter property
            - this will result in a JSONL variable being built up if this "value" is consistently a dictionary
        - if the ${value} is a list/array then it will be serialized to a JSON array and appended to the property as `\n${json-array}`
        - because the object and array types will lead to the formation of a JSONL based string, if we get a `value` which is either an empty string or undefined/null we will preserve the JSONL pattern by:
            - if the first line in the Frontmatter[`prop`] is a JSON object then we'll add '{}' otherwise '[]'

- `prepend(prop, value)`

    Prepends a value `value` to the Frontmatter property `prop`.

    > Note: behaves the same as append but in reverse; that includes putting the `\n` character _after_ the new line instead of before

- `merge(prop, value)`

    Merges `value` into the Frontmatter property `prop`. This assumes that the Frontmatter `prop` is either empty/null/undefined or an object shaped property. If it is not then we will immediately stop execution with a `InvalidAction` error.

    > **Note (semantics):**
    >
    > - performs a **shallow** object merge: the result is the top-level key union of `prop` and `value`
    > - on key collisions, the value from the new `value` overwrites the existing key
    > - if a key's value in `value` is itself an array, that array **replaces** the existing array at that key (no concatenation)
    > - if `value` is not an object (e.g. it is a string, number, boolean, or array at the top level) this yields an `InvalidAction` error
    > - if `prop` is empty/null/undefined the result is simply `value`

### Action Atomicity

Actions within a single iteration are applied **all-or-nothing**:

- Actions are applied to a **staged copy** of the post-prompt frontmatter; the live state is not mutated until every action in the list succeeds.
- Only on full success of every action is the staged copy committed as iteration N+1's incoming frontmatter.
- Any single action raising an error (`InvalidAction`, `InvalidIncrementType`, `InvalidDecrementType`, etc.) **discards the entire staged copy** — no partial mutations are persisted.
- Under `fail_fast: false`, the next iteration starts from the **pre-action** frontmatter of the failed iteration (i.e. iteration N's incoming state), not from a partially-mutated state.
- Error messages must still report **which action index failed** within the list, e.g. `InvalidIncrementType at iteration 7, action 2 of 4`.
- If the **prompt run itself** fails (non-zero exit, agent error, etc.), no actions have been staged yet — there is nothing to roll back. The iteration's frontmatter remains in its pre-action state, and `fail_fast` semantics from [Error Handling](#error-handling) determine whether the loop halts or continues. Under `fail_fast: false`, the next iteration sees the same frontmatter the failed iteration started from (and `last_output` / `last_exit_code` reflect that failure).

## Iteration Semantics

Each iteration follows a **pre-check** order:

1. **Evaluate condition** (`while` / `until`) against the current effective state (frontmatter + ambient variables for the upcoming iteration).
2. If the condition says continue, **run the prompt**.
3. **Apply actions** in the order they appear in `actions` to produce the frontmatter for the next iteration.
4. Repeat from step 1.

This is why "no action is taken when the prompt is first executed" — actions only apply _after_ each prompt run, never before the first.

### State Propagation

Iteration N+1's frontmatter equals iteration N's frontmatter **after that iteration's actions are applied**. In other words: actions applied at the end of iteration N are visible to the condition check, the prompt body, and any expressions in iteration N+1.

## Safety Cap

To guard against runaway loops, every loop has a maximum iteration count.

- **Default:** `max_iterations` defaults to **100**.
- **Per-prompt override:** set `loop.max` in the frontmatter to raise or lower this for a specific prompt.
- **Runtime override:** the CLI accepts `--max-iterations <N>`, and the env variable `CLAUDINE_MAX_ITERATIONS` provides the same override (mirroring the `--fail-fast` / `CLAUDINE_FAIL_FAST` precedent established by [sequence](../../_completed/2026-04-04-sequence/spec.md)).
- Precedence (highest to lowest): CLI flag → env var (`CLAUDINE_MAX_ITERATIONS`) → `loop.max` → default 100.

When the cap is exceeded, the loop halts immediately and surfaces a `LoopLimitExceeded` error. The error message must include the cap value and the prompt path so the user can diagnose quickly.

### Migration Note

This feature renames sequence's env vars to a namespaced form to avoid collisions with bare globals used by other tools:

- `FAIL_FAST` → `CLAUDINE_FAIL_FAST`
- `CLAUDINE_MAX_ITERATIONS` is introduced fresh by this feature (sequence does not define a `MAX_ITERATIONS` env var).

The bare `FAIL_FAST` name continues to work for **one release** with a deprecation warning logged once per process, after which it is removed. When a process reads the legacy `FAIL_FAST` env var, Claudine emits a `tracing::warn!` once per process pointing the user at the new `CLAUDINE_FAIL_FAST` name (matching the legacy single-brace template-placeholder deprecation in `claudine/lib/src/dispatch/template.rs`). See [sequence](../../_completed/2026-04-04-sequence/spec.md) for the originating precedent.

## Ambient Variables

On each iteration, the looping engine injects a set of read-only ambient variables alongside the (mutated) frontmatter. These are available both inside the prompt body (via `{{ ... }}` interpolation) and inside `while` / `until` conditional expressions.

- `iteration` — 1-indexed counter for the current iteration. `1` on the first iteration.
- `is_first` — `true` on iteration 1, `false` thereafter.
- `is_last` — `true` when this iteration's post-action condition will terminate the loop, **or** when this iteration is the one at which `max_iterations` is about to be hit. `false` otherwise.

    > **Note:**
    >
    > Because `is_last` requires looking ahead at the post-action condition, it is computed by speculatively applying actions to a copy of the state and re-evaluating the condition. The user-visible state is unchanged by this lookahead.

- `last_output` — the agent's stdout from the **previous** iteration. Empty string (`""`) on iteration 1.
- `last_exit_code` — the exit code from the previous iteration. **`0` on iteration 1** (chosen over "unset" to keep the variable always-defined and numeric for arithmetic conditions).

> **Note:**
>
> Output collection across iterations is DIY. To accumulate transcripts, wire something like `append(log, "{{last_output}}")` into `actions`. The looping engine does not implicitly accumulate output.

> **Note:**
>
> Ambient variables shadow same-named frontmatter keys for the duration of the iteration. Setting (or attempting to `set`) a frontmatter key with the same name as an ambient variable yields an `InvalidAction` error, mirroring the existing rule for `loop` and `replace`.

## Examples

See the `examples/` directory in this feature folder for working prompt files:

- `counter-loop.md` — basic `while` loop with `increment`
- `retry-until-success.md` — `until` loop with `fail_fast: false`
- `accumulate-log.md` — `append` action building JSONL

## Error Handling

Loops inherit the `fail_fast` semantics established by [sequence](../../_completed/2026-04-04-sequence/spec.md):

- **Default behavior is fail-fast**: if a single iteration's prompt or any of its actions fails, the loop halts immediately with a clear error.
- **Per-prompt opt-out:** set `loop.fail_fast: false` in the frontmatter to make the loop continue past per-iteration failures (still subject to `max_iterations`).
- **Runtime override:** the existing `--fail-fast <boolean>` CLI flag and `CLAUDINE_FAIL_FAST` env variable apply unchanged. When a document with `loop` is run, `CLAUDINE_FAIL_FAST` is set to `true` or `false` for the prompt environment, identical to sequence's behavior. (See [Migration Note](#migration-note) for the rename from `FAIL_FAST`.)
- All loop errors must include the **failing iteration index** (1-based) in their message — e.g. `LoopLimitExceeded at iteration 100`, `InvalidIncrementType at iteration 7`.

> **Note:**
>
> Under `fail_fast: false`, `last_exit_code` reflects the **failing iteration's** exit code so subsequent iterations' conditions can branch on the prior failure. This is the natural continuation of the ambient-variable contract; for example, a loop can retry until success with `until: "last_exit_code == 0"`.

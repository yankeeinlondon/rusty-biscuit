# Phase 4 — Stack Execution Engine: Findings

Validation-checkpoint artifact for Phase 4. Captures the executor's public
contract, the engine/runtime boundary (what Phase 5 must still wire), and one
parsing interaction discovered during execution that later phases should
address deliberately.

## What Phase 4 delivered

A self-contained stack execution engine in
`claudine/lib/src/composition/lifecycle_executor.rs`:

- `StackExecutionContext` — per-event context holding the composed
  frontmatter, the `err`/`timing`/`current` globals, the side-effect engine,
  an injectable `ShellRunner`, the `LifecycleEmitter`, and messaging/settings.
- `StackExecutionContext::execute_event(&LifecycleConfig)` — emits top-level
  communication first, then processes the typed `stack:` top to bottom.
- `LifecycleEventOutcome { control, action_error }` — the engine's result.
  `control` is the resolved [`StackControl`] that terminated the stack (if
  any); `action_error` carries an unintentional action error. `routes_to_failure(signal)`
  encodes the propagation table (setup-phase events route to `failure`).
- `StackControl` — the post-evaluation form of a lifecycle control action
  (`Stop`/`Skip`/`Error`/`Proxy`/`Retry`/`Resume`/`Requeue`) with every
  expression argument already evaluated.
- `ShellRunner` trait + `SystemShellRunner` — approved-shell execution,
  injectable so L1 tests assert dispatch without spawning processes.

### Supporting changes outside the new module

- `LifecycleEmitter` gained `emit_info`/`emit_warn` (default impls render an
  `Info`/`Warning` `Status` line to stderr — never stdout).
- `LifecycleSignal::routes_action_error_to_failure()` encodes the setup-phase
  vs terminal-phase split of the action error-propagation table.
- `LifecycleErrorInfo::from_action_failure(verb, msg)` builds the `err`
  snapshot a routed-to `failure` event observes after a setup-phase action
  error.
- `lifecycle_actions::side_effect_signature(verb)` / `is_known_side_effect(verb)`
  expose the Darkmatter side-effect verb catalog. The long-form parser
  (`build_action_from_params`) now reorders named params into the verb's
  positional signature instead of sorting them alphabetically — alphabetical
  order silently mis-ordered `http_post(url, body)` and
  `ensure_file(file, content)`.
- `audio_phases` / `tts_config_from_settings` / `AudioPhase` were made
  `pub(crate)` so the executor reuses the existing audio-ordering logic.

## Engine vs runtime boundary (what Phase 5 must wire)

Phase 4 is the **engine**: it dispatches actions and reports an outcome. It
does **not** perform the control flow those outcomes imply. Phase 5 consumes
`LifecycleEventOutcome` and acts on it:

- `StackControl::Skip` → whole-document opt-out (no provider, no `finalize`,
  no `loop`).
- `StackControl::Proxy` → fresh target-prompt run at its `initialize`.
- `StackControl::Retry`/`Resume`/`Requeue` → re-entry / queue.
- `StackControl::Error` → the explicit success→failure conversion at
  `success`/`finalize` (the engine surfaces it distinctly from an
  unintentional `action_error`; the runtime applies the transition).
- `outcome.routes_to_failure(signal)` → route a setup-phase action error to
  the `failure` event.

The existing `LifecycleRunGuard` still owns top-level emission for the legacy
four-event path. Phase 5 must reconcile the guard and the executor so a given
event's top-level communication fires exactly once (the executor's
`emit_top_level` is the intended single source going forward, and it adds the
`info`/`warn` channels the guard never emitted).

## Action error semantics (implemented)

- An action that errors with `no_error: false` stops the **whole** stack, is
  logged (`warn!`), and is reported via `action_error`.
- `no_error: true` logs and continues to the next action/item; outcome
  unchanged.
- Setup-phase events (`initialize`/`start`/`blocked`) route the error to
  `failure`; terminal-phase events (`success`/`failure`/`finalize`/`loop`)
  leave the composition outcome unchanged.
- A `when:` clause that fails to evaluate logs a warning and is treated as
  non-matching (a malformed guard cannot fire an unintended action).
- Short-form side-effect verbs (e.g. `ensure_file('@x')`) parse as
  expression-function actions; the executor routes any verb in the
  side-effect catalog to the engine via `is_known_side_effect`.

## Discovered interaction — long-form bare-word values (out of scope here)

The spec's canonical long-form side effect is:

```yaml
- action: set_frontmatter
  file: "@spec.md"
  prop: "status"
  value: "in-progress"
```

`value_to_expr` (Phase 2) parses each long-form string value as a Darkmatter
expression first, falling back to a string literal only when the parse fails.
That heuristic mis-handles single-token literals:

- `"status"` parses as `Variable("status")` → evaluates to the (absent)
  frontmatter key → `null` → empty string.
- `"in-progress"` parses as `Binary(Sub, Variable("in"), Variable("progress"))`
  → evaluation error.

So the spec's documented long-form example would write to key `""` with an
evaluation error rather than `status: in-progress`. The **short form with
quoted args** (`set_frontmatter('@spec.md', 'status', 'in-progress')`) is
unaffected — quoted args parse as string literals — and is the path Phase 4's
side-effect tests exercise.

This is a Phase 2 parsing-semantics decision with a wide blast radius (the
undefined-variable validator and several Phase 2/3 tests depend on the current
greedy-parse behavior), so it was **not** changed in Phase 4 (surgical-scope
rule). It should be resolved deliberately — most likely in Phase 7
(backward-compatibility / UX polish) or as a dedicated follow-up — by making
long-form scalar values literal-leaning unless they are a namespaced
reference (`ctx.`/`env.`/`doc.`), an interpolation span, or a structured
expression. The engine itself needs no change; it evaluates whatever `Expr`
the parser produces.

## Validation

- 18 new L1 tests in `lifecycle_executor` cover: top-level-before-stack
  ordering, `when` true/false/omitted, scalar + array action ordering,
  control-action termination, communication rendering paths
  (say/info/warn/message/effect/stderr), shell dispatch + `on_error`,
  setup-vs-terminal error propagation, `no_error`, errored-action stops the
  stack, explicit `Error` control surfacing, `retry(N)` shorthand,
  side-effect short-form + long-form-reorder, `err` global visibility in a
  `failure` stack `when:`, and literal `{{ }}` interpolation.
- `just test` (claudine area): 1673 passed (one pre-existing unrelated flaky
  test in `commands::wrap::exec::exit`).
- `just lint` (claudine area): clean.

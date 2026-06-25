---
status: draft
depends_on: ../2026-05-12-lifecycle/spec.md
reviewed: false
---

# Lifecycle Actions: Two Forms (Positional and Key/Value)

## Introduction

Lifecycle stacks currently accept **three** ways to write a single action inside
an event's `action:` value:

1. **Short form** — a scalar string `verb(args)`, e.g. `success("…")`,
   `shell(git push)`, `set_frontmatter('s.md', 'status', 'done')`.
2. **Long form** — an object with an explicit `action:` verb-discriminator key
   plus named parameter keys, e.g. `{ action: shell, command: "git push", no_error: true }`.
3. (Implicitly) the stack-item-level scalar `action: <verb>` with the verb's
   parameters as **sibling keys** of `action:`.

This feature reduces the surface to **two** forms — **positional** and
**key/value** — and removes short form. The change is motivated by a real
authoring failure: the intuitive YAML shape an author reaches for first,

```yaml
action:
    - success: "review passed"
    - effect: "small-group-cheer"
```

is not currently a valid form. It parses each element as a long-form object,
finds no `action:` key, and fails with `long-form action object must have an
'action' key` (`composition/lifecycle.rs:1670`). The diagnostic is accurate but
the grammar is the problem: the shape the author guessed should simply be legal.

This spec makes that shape — the **positional** form — a first-class citizen,
and in doing so unlocks a single, uniform evaluation rule for the whole lifecycle
surface.

## Motivation

- **Remove the parameter-name memory burden.** Key/value form requires the author
  to know each verb's named parameters (`message:`, `command:`, `on_error:`,
  `file:`/`prop:`/`value:`, …). Until a language server assists with this, that is
  a large ask. Positional form needs zero knowledge of parameter names — the verb
  is the key, and the value is its argument(s).
- **The intuitive shape should be the valid shape.** The form that produced the
  motivating parse error is exactly the form authors expect to work.
- **One evaluation rule.** Short form is the *only* reason the lifecycle surface
  needs a comma-separated **expression** sub-grammar (see
  [Why removing short form enables one rule](#why-removing-short-form-enables-one-rule)).
  Removing it lets the entire surface default to **literal text with `{{ … }}`
  interpolation**, with the three boolean-predicate keys (`when`, `until`,
  `while`) as the only expression-evaluated exception.
- **Visual consistency with the event header.** Positional form is already how the
  top-level notification fields read (`failure: { stderr: …, message: …, effect: … }`),
  so a stack stops looking like a different language from the event it lives under.

## Why removing short form enables one rule

`verb(args)` cannot be **both** literal-by-default **and** multi-argument:

- For a single-argument verb, the whole parenthesized body can be one literal
  (today's `message(…)` behavior: no comma splitting, no quoting —
  `is_single_text_arg_verb`, `composition/lifecycle.rs:1789`).
- For a multi-argument verb (`set_frontmatter(a, b, c)`), the commas **must** mean
  "argument separator," which forces those args into an expression/quoting grammar
  (`split_action_args`, comma-separated expressions where a bare token is a
  *variable* and multi-word literals must be quoted).

A YAML array moves the separation into YAML itself
(`set_frontmatter: ["a", "b", "c"]`), so each element stays pure literal text plus
`{{ … }}`. Removing short form therefore is not mere cleanup — it is the
precondition that lets the literal-default rule be **universal** instead of a rule
with an expression sub-grammar bolted onto the multi-argument verbs.

## The single evaluation rule

> **Every value in a lifecycle action is literal text. Use `{{ … }}` to inject a
> variable or an expression. The only expression-evaluated keys in the entire
> lifecycle surface are the boolean predicates `when`, `until`, and `while`.**

Consequences:

- A bare token is literal text, never a variable: `set_frontmatter: ["s.md", "status", "done"]`
  writes the literal string `done`, never the value of a variable named `done`.
- Interpolation injects computed values: `message: "review {{iteration}} passed"`,
  `set_frontmatter: ["s.md", "status", "{{ ctx.area }}"]`.
- **Typed values** follow the existing whole-value rule: an argument whose trimmed
  content is exactly one `{{ expr }}` span resolves to the expression's *typed*
  value, not a string. So `set_frontmatter: ["s.md", "ready", "{{ true }}"]` writes
  a boolean `true`; `"{{ 3 }}"` writes a number `3`; `"3"` writes the string `"3"`.
  This matches Darkmatter's whole-value frontmatter expansion contract.
- `when` / `until` / `while` are unchanged: they parse as boolean expressions, not
  literals.

## The two forms

### Positional

An object whose **single key is a known verb**. The value carries the argument(s):

- **Scalar value** → one argument:
  ```yaml
  - message: "review {{iteration}} passed"
  - effect: "small-group-cheer"
  - proxy: "other-prompt.md"
  ```
- **Array value** → positional arguments, zipped against the verb's canonical
  signature (`side_effect_signature`, `composition/lifecycle_actions.rs:353`):
  ```yaml
  - set_frontmatter: ["state.md", "status", "production ready"]
  - append_line: ["log.md", "review {{iteration}} done"]
  ```
  An **empty array** (`[]`) is the zero-element case of this rule — i.e. zero
  arguments — so it is equivalent to a null value for no-arg verbs.
- **Null value, empty array, or bare verb-name string** → zero arguments (for
  no-arg and all-optional-arg verbs). All three spellings are equivalent:
  ```yaml
  - stop:             # null value
  - stop: []          # empty array
  - stop              # bare verb-name string
  - error:            # error with no reason (error's one arg is optional)
  ```
  The bare verb-name string (no value, no key) is the **only** surviving string-
  element form — a more natural reading for no-arg control verbs than `stop:`. A
  zero-argument spelling is valid only for verbs whose arguments are all optional or
  zero; supplying zero arguments to a verb that requires one (e.g. `proxy`,
  `resume`, by any of the three spellings) is a typed wrong-arity error. A string
  element that contains `(` is treated as removed short form and produces the did-
  you-mean removal error (see [Scope: Remove](#scope-remove-short-form)), never a
  zero-arg action.

Positional is **single-key only**. Optional named parameters that have no
positional slot (`route`, `on_error`, `no_error`) are **not** expressible in
positional form — use key/value form for those.

### Key/Value

An object with an explicit **`action:` verb-discriminator key** plus named
parameter keys. Unchanged from today's long form:

```yaml
- action: shell
  command: "git push origin HEAD"
  on_error: "push failed"
  no_error: true
- action: set_frontmatter
  file: "state.md"
  prop: "status"
  value: "production ready"
```

Key/value is the form to reach for when you want self-documenting parameter names
or an optional named parameter (`route`, `on_error`, `no_error`).

### Disambiguation

The two forms are distinguished structurally, with no ambiguity:

| Condition | Form |
|-----------|------|
| Array element is a bare string naming a known verb (no `(`) | **positional**, zero arguments |
| Array element is a string containing `(` | error: removed short form, did-you-mean positional rewrite |
| Object has an `action:` key | **key/value** |
| Object has exactly one key, and that key names a known verb (and is not `action`) | **positional** |
| Object has exactly one key that is **not** a known verb | error: unknown verb / did-you-mean |
| Object has multiple keys but no `action:` key | error: ambiguous — use key/value (`action:`) or a single-key positional |
| Object key value is a map (not scalar/array/null) | error: positional values are scalar, array, or empty |

`action` is never itself a verb, so a positional key never collides with the
key/value discriminator. Verbs that are also key/value parameter names (e.g.
`message`) do not collide either, because key/value form **requires** the
`action:` key; a lone `{ message: "…" }` unambiguously means positional `message`.

### Both forms appear in the same places

A stack item's `action:` value may be:

- a single positional map (`action: { success: "…" }`),
- a single key/value map (`action: { action: shell, command: "…" }`),
- or an **array** mixing positional and key/value elements.

A single action need not be wrapped in an array-of-one.

## Scope: Remove (short form) {#scope-remove-short-form}

Short form — a scalar string of the shape `verb(args)` — is removed everywhere it
is accepted. (A bare verb-name string with **no** parentheses is **not** short form;
it survives as the zero-argument positional form — see
[Positional](#positional).)

- The scalar-string `verb(args)` branch of the stack `action:` value
  (`composition/lifecycle.rs:1507`, `parse_scalar_action`).
- The `verb(args)` string-element branch inside an `action:` array
  (`composition/lifecycle.rs:1527`); replaced by the bare-verb-name zero-arg form.
- `parse_short_form_action` (`composition/lifecycle.rs:1729`) and the
  comma-separated expression argument grammar it depends on for multi-argument
  verbs (`split_action_args`, `parse_action_arg`), to the extent no kept surface
  needs them.
- **Resolved (clarification #3): remove** the stack-item-level `action: <verb>`
  scalar **with sibling parameter keys** path. `action: <verb>` + siblings is
  key/value written without nesting; consolidating on the explicit `action:`-key
  **object** form keeps the two forms visually distinct (positional = no `action:`
  key; key/value = has an `action:` key).

`is_single_text_arg_verb` may be retained as the value-arity classifier for
positional scalar-vs-array validation if convenient, but it no longer gates a
parenthesized-body literal path.

## Scope: Keep / Add (parsing)

- **Positional parser** — new branch in the `action:`-array element match and the
  single-map `action:` value: detect single-verb-key objects, classify the value
  (scalar → 1 arg; array → N args; null/empty → 0 args), and build the action via
  the existing `build_action_from_params` / signature-zip machinery.
- **Array→positional zip** — reuse `side_effect_signature` to map an array to the
  verb's positional parameter order. A wrong-arity array is a typed error that
  names the expected count and the verb's parameter names.
- **Key/value parser** — `parse_long_form_action_object`
  (`composition/lifecycle.rs:1664`) is unchanged.
- **All action families** remain reachable through both forms: communication
  (`CommunicationChannel::from_verb`), `shell`, side-effects
  (`side_effect_signature`), and lifecycle control
  (`parse_lifecycle_control_long`). Control verbs in positional form use the
  scalar/null value as the single optional argument (`error: "reason"`,
  `retry: 3`, `proxy: "x.md"`, `stop:`), or a bare verb-name string for the
  zero-argument case (`stop`, `skip`).
- **Cardinality and placement checks** — at most one lifecycle control action,
  last in the stack; per-event placement matrix (`is_valid_for`). Unchanged
  (`composition/lifecycle.rs:1572-1606`).
- **Late-binding / leak guards** — event-time DM2 interpolation, the surviving-span
  leak guard, and the err-placement scan apply to positional and key/value action
  strings identically. Unchanged.

## Decisions (resolved during clarification)

1. **Short-form removal — hard removal.** `verb(args)` short form is rejected
   immediately with a typed, did-you-mean `CompositionError` that prints the
   positional rewrite (`success("x")` → `success: "x"`;
   `set_frontmatter('a','b','c')` → `set_frontmatter: ["a","b","c"]`). No
   deprecation window. This is breaking; existing prompt files and lifecycle/
   composition doc examples that use short form must be ported as part of this
   change (see [Migration](#migration)).

2. **Zero/optional-arg control verbs — null value, empty array, *and* bare verb-name
   string.** `- stop:` (null value), `- stop: []` (empty array), and `- stop` (bare
   verb-name string) are all equivalent "zero arguments"; the bare-string reading is
   the most natural for no-arg control verbs, and the empty array is the zero-element
   case of the array rule. A scalar value supplies the single optional argument for
   `error`/`retry` (`error: "reason"`, `retry: 3`). Any zero-argument spelling is
   valid only for verbs whose arguments are all optional or zero; supplying zero
   arguments to a verb that requires one is a typed wrong-arity error, and a string
   containing `(` is the removed-short-form error, never a zero-arg action.

3. **`action: <verb>` + sibling-keys path — removed.** Consolidated on the explicit
   `action:`-key object form for key/value, so positional (no `action:` key) and
   key/value (has `action:` key) stay visually distinct. See
   [Scope: Remove](#scope-remove-short-form).

4. **Typed argument escape hatch — confirmed.** A positional or named argument whose
   trimmed content is exactly one `{{ expr }}` span resolves to the expression's
   typed value (bool/number/null), matching Darkmatter's whole-value frontmatter
   rule, so side-effects can write non-string frontmatter values.

## Migration

- **Breaking** change to the action grammar. A document using `verb(args)` short
  form after this change must produce a typed, actionable `CompositionError`
  pointing at the positional (or key/value) rewrite — not a silent ignore and not
  a generic unknown-field error.
- The diagnostic should reuse the frontmatter-excerpt renderer
  (`composition::FrontmatterExcerpt`) so the offending line is highlighted in
  TTY-capable output and stays escape-free at `ColorDepth::None`.
- Rewrite every short-form example in the lifecycle topic doc
  (`claudine/docs/topics/lifecycle.md`), the composition topic doc, and the
  claudine skill docs to positional/key/value. Update the
  "Action Forms" section to describe exactly two forms and the single evaluation
  rule.
- Port internal prompt files that use short form to positional form (including
  `prompts/review-feature.md`, the motivating file).

## Acceptance Criteria

- The motivating shape parses and runs:
  ```yaml
  success:
    stack:
      - when: "frontmatter(review_file,'ready') == true"
        action:
          - success: "review {{iteration}} in {{ctx.area}} is production ready"
          - message: "✅ review #{{iteration}} for {{parent_dir(spec)}} completed"
          - effect: "small-group-cheer"
  ```
- Positional array form dispatches multi-argument side-effects in canonical order:
  `set_frontmatter: ["s.md", "status", "done"]` is equivalent to the key/value form
  with `file`/`prop`/`value`.
- A whole-value `{{ expr }}` positional argument writes a typed value
  (`set_frontmatter: ["s.md", "ready", "{{ true }}"]` writes boolean `true`).
- All action families (communication, shell, side-effect, lifecycle control) are
  reachable through both positional and key/value forms; control verbs accept their
  optional argument as a scalar value, and zero arguments as a null value
  (`- stop:`), an empty array (`- stop: []`), or a bare verb-name string (`- stop`).
- A bare verb-name string requiring an argument (e.g. `- proxy`) is a typed
  wrong-arity error, not a zero-arg action.
- Short form (`verb(args)` scalar string — any string element containing `(`) is
  rejected with a typed, did-you-mean `CompositionError` carrying source path, the
  offending text, and the positional rewrite; the error includes the frontmatter
  excerpt/highlight in TTY output and is escape-free in non-color output. A bare
  verb-name string (no `(`) is **not** rejected.
- Disambiguation matches the table in [Disambiguation](#disambiguation): an object
  with an `action:` key is key/value; a single known-verb key is positional;
  unknown-verb single keys and multi-key no-`action:` objects yield typed errors.
- Cardinality, ordering, and per-event placement checks for lifecycle control
  actions are unchanged.
- `when` / `until` / `while` continue to evaluate as boolean expressions; no other
  lifecycle key evaluates as an expression.
- Docs (lifecycle topic, composition topic, claudine skill) describe exactly two
  forms and the single evaluation rule; no doc or example references `verb(args)`
  short form.

## Test Strategy

- **L1 — positional parsing:** scalar value → 1 arg; array value → N args zipped to
  signature; null/empty value → 0 args; bare verb-name string → 0 args. Cover
  communication (`message`, `effect`, `stderr`), shell, a 3-arg side-effect
  (`set_frontmatter`), a 2-arg side-effect (`append_line`), and control verbs
  (`stop`, `error`, `retry`, `proxy`), including `- stop:`, `- stop: []`, and
  `- stop` all parsing to a zero-arg `stop`.
- **L1 — typed args:** whole-value `{{ true }}` / `{{ 3 }}` resolve to typed
  bool/number; plain `"3"` stays a string.
- **L1 — disambiguation:** `{ action: shell, command }` → key/value;
  `{ shell: "git push" }` → positional; `{ notaverb: "x" }` → typed unknown-verb
  error; `{ message: "x", route: "y" }` (no `action:`) → typed ambiguous error;
  `{ message: { … } }` → typed non-scalar-value error.
- **L1 — short-form rejection:** `success("x")`, `shell(git push)`, and
  `set_frontmatter('a','b','c')` each yield the did-you-mean removal error with the
  positional rewrite; the excerpt highlights the offending line in TTY output. A
  bare `stop` is accepted (zero-arg), while a bare `proxy` (requires an argument)
  is a wrong-arity error — confirming bare-string is gated on verb arity, not the
  short-form path.
- **L1 — arity errors:** `set_frontmatter: ["a"]` and `message: ["a","b"]` yield
  typed wrong-arity errors naming the expected count.
- **L1 — predicate exception:** `when` / `until` / `while` still evaluate as
  boolean expressions; a positional action value with a bare token stays literal.
- **L2 — end-to-end compose:** a `success` stack mixing positional communication,
  a positional multi-arg `set_frontmatter`, and a key/value `shell` action runs
  through `compose`, with event-time interpolation (`{{iteration}}`, `{{err.msg}}`)
  resolving correctly.
- **Regression sweep:** `rg` confirms no remaining `verb(args)` short-form examples
  in prompt files or docs, and no dangling references to removed parsing symbols.

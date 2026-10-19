---
area: darkmatter
status: draft-spec
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
implemented: false
created: 2026-10-16
owner: Ken Snyder <ken@ken.net>
packages:
    - darkmatter
    - dmls
    - claudine
---

# One Meaning for `||`

## Problem

The same expression gives a different answer depending on where it is written.
In `{{ … }}` interpolation, `||` is a fallback: `a || b` is `a` when `a` is
truthy, otherwise `b`. In a condition (`when:`, `::block when="…"`, a loop's
`while`/`until`, a transclusion `when`), `||` is lowered to `or(a, b)`, which
returns a boolean.

That difference is silent whenever the result of `||` is used as a value
rather than as the final truth test. It was found in a shipped prompt on
2026-10-16:

```yaml
initialize:
    - when: "length(frontmatter(plan, 'open_questions') || []) > 0"
      then:
          - error: "the plan has unresolved open questions"
```

With one open question in the plan, the guard never fired:

| Step | `{{ … }}` | `when:` (today) |
| --- | --- | --- |
| `frontmatter(plan, 'open_questions') \|\| []` | `["Which corpus?"]` | `true` |
| `length(…)` | `1` | `0` (`length` maps booleans to `0`) |
| `… > 0` | `true` | `false` |

No error, warning, or diagnostic was produced at any step, and the identical
text inside `{{ }}` gives the right answer. An author who has learned `x || []`
as the idiom for a missing list, from interpolation, the docs, and every
shipped prompt, has no way to see that it means something else in a
condition.

The mode split is documented (the operator table in
[Darkmatter Expressions](../../docs/topics/darkmatter-expressions.md#interpolation-vs-condition-mode)),
but documentation is not a defense against a trap that produces plausible
wrong answers. The same page also states that both modes lower `||` into an
`or(…)` call; interpolation actually builds a `Fallback` node, so that
sentence is wrong today.

## Goal

`||` is logical OR on every surface, in the value-returning form used by
JavaScript, Python's `or`, Ruby, and Lua: it returns **the operand that decided
the result** -- the first truthy operand, otherwise the last operand --
short-circuiting left to right. This is today's interpolation behavior,
unchanged. It is not a separate "fallback" operator: as a truth value it agrees
with boolean OR on every input, and returning the deciding operand is what makes
the `x || default` idiom work.

| Expression | Value on every surface | As a condition |
| --- | --- | --- |
| `"foo" \|\| "bar"` | `"foo"` | true |
| `"" \|\| "bar"` | `"bar"` | true |
| `false \|\| 0` | `0` | false |

## Why this is safe

A condition only ever asks whether its final value is truthy. For any `a` and
`b`, `a || b` under fallback semantics is truthy exactly when `or(a, b)` is
`true`, and both evaluate `b` only when `a` is falsy. So:

- **every condition whose `||` is the outermost test keeps its outcome**, as
  does any `||` under `!`, `&&`, or a ternary condition, because those consume
  the operand as a truth value too
- **only a `||` whose result is consumed as a value changes**: a function
  argument (`length(x || [])`) or a comparison operand (`(a || b) == true`)

A repository-wide search on 2026-10-16 of `when`, `while`, and `until`
expressions in Markdown, YAML, and Rust (excluding `_completed/` specs and
frozen fixtures) found no expression of the second kind. The one that existed,
in `prompts/_implement/implement-plan.md`, was rewritten to `!is_empty(…)` the
same day. The change therefore fixes the trap without altering any existing
result, so it needs no deprecation period, transition warning, or opt-in.

## Decisions

### D1 — `||` is one AST node, renamed `LogicalOr`

The parser lowers `||` to a dedicated node in every context; condition parsing
no longer produces an `or(…)` call for infix `||`. The node is today's
`Expr::Fallback` (and `SpannedExprKind::Fallback`), renamed `LogicalOr` with
fields `left`/`right`, so its name says what the operator is. The evaluator's
existing arm is the only implementation of `||`.

A node is kept rather than lowering to a function call because it carries what
a call cannot without special cases:

- **operand-specific absence scope:** only the left operand is an absence
  check, and the right operand inherits the enclosing scope so a chain
  `a || b || "d"` handles `b` too (see D3); a call's absence handling is one
  switch for all its arguments (`AbsenceScope::call_argument`)
- **a value result:** the explicit `or(…)` function returns a boolean (D4);
  reusing it would change that contract
- **source fidelity:** the tree matches what the author wrote, which the
  nested-span lint relies on when it compares a reparsed rewrite with the
  original

Short-circuiting is not a reason: the evaluator already evaluates `and`/`or`
calls lazily.

### D2 — The parse mode is removed

`||` is the only thing `ParseMode` changes; the lexer's `Token::OrOr` exists
only to carry the condition meaning, and the nested-span lint carries a
workaround for the difference (it refuses to suggest a rewrite that would move
a fallback into condition grammar). With one meaning for `||` the mode has no
remaining purpose, so it is removed rather than left as a no-op:

- `ParseMode`, `Token::OrOr`, `Lexer::with_mode`, `parse_condition`, and
  `parse_condition_spanned` are deleted; their callers use `parse` and
  `parse_spanned`
- the callers are in `darkmatter` (conditions, transclusion conditions,
  directive targets, frontmatter shell expansion, prepared expressions, lint,
  schema format/convert), `dmls` (frontmatter diagnostics and providers,
  overlay expressions, nested-span diagnostics), and `claudine` (dispatch
  matcher and runner, looping, and the lifecycle parser/validator/bindings);
  about 58 non-test call sites in all
- the nested-span lint drops its per-mode reparse; the
  `fallback_span_in_condition_dialect_has_no_suggestion` case is now offered
  a suggestion, and its test is rewritten to assert that
- a bare `|` keeps its error, worded once: "Unexpected '|'. Use '||'."

"Condition" remains a word for *where* an expression is used (a truth test)
and for the `ConditionError` type that reports failures there. It stops being
a name for a separate grammar, and prose that says "condition dialect" or
"condition mode" is rewritten.

### D3 — Absence follows the interpolation rule

Today the operands of a condition `||` are ordinary reads: an undeclared
property on either side gets the advisory `dm.expression.undeclared_property`.
In interpolation, the left operand of `||` is an absence check and gets no
advisory, because `x || "default"` is the documented way to handle a missing
`x`.

The interpolation rule applies everywhere. The cost is that an undeclared,
misspelled bare variable on the **left** of a condition `||`
(`when: "spce || plan"`) no longer gets the advisory. That is accepted: a rule
that depends on the surface is the trap this fix removes, the right operand is
still reported, a left operand that is anything other than a bare variable
(`!file_exists(x)`, a comparison, a call) is unaffected, and schema-declared
names are validated regardless.

### D4 — `&&` stays boolean; `or()` and `and()` are unchanged

`&&` keeps returning `true`/`false` on every surface, and the explicit function
forms `or(…)` and `and(…)` keep returning booleans. `or(…)` is how an author
asks for a boolean OR explicitly.

This leaves `&&` and `||` asymmetric, which is a deliberate decision rather
than an oversight. A value-returning `&&` (`a && b` returns `a` when it is
falsy, otherwise `b`) would leave every condition's outcome unchanged, but it
would change interpolated output: `{{ x && 'yes' }}` renders `true` today and
would render `yes`. That is a rendering change with no defect motivating it,
and it would need its own audit of interpolations. `&&` already means the same
thing on every surface, so it is not part of this trap. `&&` continues to lower
to an `and(…)` call; giving it its own node is not required by this fix.

### D5 — Helper type leniency is out of scope

`length(true)` returning `0` is what turned this into a silent wrong answer,
but with `||` unified a well-formed fallback can no longer feed a boolean to
`length`. Making helpers reject wrong-type input is a separate decision.

## Acceptance Criteria

1. `||` gives the same value on every surface. Each row holds in `{{ … }}`, in
   a Darkmatter `::block when="…"` (used as its truth value), and in a
   Claudine lifecycle `when:`:

   | Expression | State | Value |
   | --- | --- | --- |
   | `items \|\| []` | `items: [1, 2]` | `[1, 2]` |
   | `items \|\| []` | `items` absent | `[]` |
   | `length(items \|\| []) > 0` | `items: ["q"]` | `true` |
   | `length(items \|\| []) > 0` | `items` absent | `false` |
   | `a \|\| b` | `a: "x"`, `b: true` | `"x"` |
   | `a \|\| b` | `a: false`, `b: 0` | `0` |
   | `true \|\| unknown_fn()` | — | `true`, and `unknown_fn` is never called |

2. A Claudine lifecycle `when: "length(frontmatter(plan, 'open_questions') || []) > 0"`
   fires when the plan lists one open question and does not fire when the
   property is absent.
3. Every existing test of condition outcomes passes unchanged, apart from
   tests that assert the AST shape (`FunctionCall { name: "or" }` from infix
   `||`, or the `Fallback` name) or the parse mode. Those are rewritten to
   assert `LogicalOr`, never deleted.
4. `ParseMode`, `Token::OrOr`, `parse_condition`, and `parse_condition_spanned`
   no longer exist, and a workspace build with every feature enabled passes.
5. A left-operand undeclared bare variable in a condition `||` produces no
   `dm.expression.undeclared_property` advisory, and a right-operand one still
   does (D3).
6. The nested-span lint offers its suggestion for `a == "x {{ y || 'd' }}"`
   wherever it appears.

## Documentation

`docs/` pages describe the result, not the change:

- `darkmatter/docs/topics/darkmatter-expressions.md`: replace the
  "Interpolation vs. Condition Mode" section with a short statement that `||`
  is logical OR returning the deciding operand on every surface, with the
  three-row table from the Goal and one example of `x || []` in a condition;
  call it "Logical OR" (not "Fallback") in the precedence list and operator
  tables; correct the parsing summary that says both modes lower `||` into
  `or(…)`; state that `or(…)`/`and(…)` and `&&` return booleans and why `&&`
  differs (D4)
- `darkmatter/docs/topics/schemas/parsing/{index,grammar,lexing}.md` and
  `schema-targeting.md`: remove the parse-mode grammar branches and token, and
  rename the `fallback` grammar rule to `logical_or`
- the operator descriptor in `darkmatter/lib/src/markdown/compose/expression/semantics.rs`
  (shown in DMLS hover): named "Logical OR", described as "returns the operand
  that decided the result: the first truthy operand, otherwise the last", with
  no mode clause
- `claudine/docs/topics/flow-control/lifecycle.md` and
  `claudine/docs/topics/state-management/expression-engine.md`: remove
  "condition dialect" wording
- the `darkmatter` and `claudine` skills, wherever they repeat the mode split

## Out of Scope

- helper functions rejecting wrong-type arguments (D5)
- `&&` returning an operand (D4)
- Claudine hook-action `when` (`dispatch/runner/mod.rs`), which skips an action
  with a warning when its condition fails to parse or evaluate; that is a
  separate decision about hook dispatch, unaffected by this change

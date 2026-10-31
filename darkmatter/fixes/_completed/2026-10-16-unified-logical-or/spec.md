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
implemented: true
created: 2026-10-16
owner: Ken Snyder <ken@ken.net>
packages:
    - simplified-schema
    - darkmatter
    - dmls
    - claudine
implemented_by: "claude/opus"
review_iterations: 2
completed: true
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
same day. The search was repeated on 2026-10-30 (101 condition lines
containing `||`) with the same result. The change therefore fixes the trap
without altering any existing result, so it needs no deprecation period,
transition warning, or opt-in. Shipped prompts keep changing, so the
implementation repeats the search before it lands.

Static analysis already agrees: the shared narrowing facts
(`assume` in `darkmatter/lib/src/markdown/compose/expression/scope/facts.rs`)
treat `a || b` exactly as `or(a, b)` when it is used as a truth test. Unifying
the operator brings the evaluator into line with what static analysis already
assumes.

## Decisions

### D1 — `||` is one AST node, renamed `LogicalOr`

The parser lowers `||` to a dedicated node in every context; condition parsing
no longer produces an `or(…)` call for infix `||`. The node is today's
`Expr::Fallback` (and `SpannedExprKind::Fallback`), defined in
`simplified-schema/lib/src/expression/ast.rs` and re-exported by Darkmatter,
renamed `LogicalOr` with fields `left`/`right`, so its name says what the
operator is. The evaluator's existing arm
(`darkmatter/lib/src/markdown/compose/expression/mod.rs`) is the only
implementation of `||`.

A node is kept rather than lowering to a function call because it carries what
a call cannot without special cases:

- **operand-specific absence handling:** the left operand is an absence check,
  and the right operand inherits the enclosing scope, so a chain
  `a || b || "d"` handles `b` too (see D3). The operands of an `or(…)` call
  are neither absence checks nor consumed values; they only inherit the
  enclosing scope (see the `absence.rs` module docs), so `or(x, "d")` does not
  handle `x`. Both forms already give a later operand the earlier operands'
  false-side facts
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
  `parse_condition_spanned` are deleted from
  `simplified-schema/lib/src/expression/` and from Darkmatter's re-exports
  (`darkmatter/lib/src/markdown/compose/expression/mod.rs`); their callers use
  `parse` and `parse_spanned`
- the callers are every use of those names in the workspace, in four crates
  (re-count before implementing; the list grows):
    - `simplified-schema`: schema `format` and `simplified/convert`
    - `darkmatter`: conditions, call precheck, page-block guards,
      `::map`/`::filter` parser and scopes, frontmatter shell expansion,
      prepared expressions, call binding, lint, and the lifecycle
      parser/validator (`darkmatter/lib/src/lifecycle/`)
    - `dmls`: frontmatter diagnostics, DSL and semantic-token providers,
      overlay expressions, nested-span diagnostics
    - `claudine`: dispatch matcher, runner, and templates; looping; sequence
      grammar, expressions, and preflight; lifecycle bindings, rename, binding
      checks, and executor; benches
- public signatures that take a mode lose it: `lint_expression(source, mode)`
  becomes `lint_expression(source)`
- the `expression` schema format validated "under either dialect" through a
  condition-mode parse because that parse accepted a superset; it now uses
  `parse`, and the reserved `expression(condition)` form is no longer reserved:
  with one grammar a parameterized form has nothing to select, so
  `expression(…)` stays rejected without a "reserved" explanation
- the nested-span lint drops its per-mode reparse; the
  `fallback_span_in_condition_dialect_has_no_suggestion` case is now offered
  a suggestion, and its test is rewritten to assert that
- a bare `|` keeps its error, worded once: "Unexpected '|'. Use '||'."
  (today it has two wordings, one per mode)

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
   assert `LogicalOr`, never deleted. This covers static verdicts as well as
   runtime outcomes: the passive call check (`call_check.rs`), static
   truthiness in frontmatter interpolation, preflight collection, and the DMLS
   overlay already handle the `Fallback` node, and they now see it where they
   used to see an `or(…)` call. Every shipped condition keeps its passive
   call-check verdict, checked by the passive shipped-artifact corpus test.
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
- `simplified-schema/docs/topics/expression-syntax.md`: remove the
  `parse_condition`/`parse_condition_spanned` rows and the `ParseMode` note
  from the API table, replace the two-mode comparison table, and drop the
  "condition mode accepts a superset" explanation of schema checks
- `darkmatter/docs/topics/schemas/definition.md`: the `expression` format's
  "two dialects" semantics paragraph (which also wrongly says the dialects
  disagree on `&&`) and the `expression(condition)` reservation in its
  limitations list (D2)
- `darkmatter/docs/topics/schemas/parsing/{index,grammar,lexing}.md` and
  `darkmatter/docs/topics/schemas/schema-targeting.md`: remove the parse-mode
  grammar branches and token, and rename the `fallback` grammar rule to
  `logical_or`
- the operator descriptor in `darkmatter/lib/src/markdown/compose/expression/semantics.rs`
  (shown in DMLS hover): named "Logical OR", described as "returns the operand
  that decided the result: the first truthy operand, otherwise the last", with
  no mode clause; likewise the precedence entry
  `("Logical OR / Fallback", "`||` (mode-dependent)")` in
  `simplified-schema/lib/src/expression/parser.rs`
- the module docs of `simplified-schema/lib/src/expression/mod.rs` and
  `darkmatter/lib/src/markdown/compose/expression/mod.rs`, which describe the
  two parse modes
- `claudine/docs/topics/flow-control/lifecycle.md` and
  `claudine/docs/topics/state-management/expression-engine.md`: remove
  "condition dialect" wording
- the `darkmatter` skill (including `expressions.md`, which names the
  `Fallback` node) and the `claudine` skill, wherever they repeat the mode
  split

## Out of Scope

- helper functions rejecting wrong-type arguments (D5)
- `&&` returning an operand (D4)
- Claudine hook-action `when` (`dispatch/runner/mod.rs`), which skips an action
  with a warning when its condition fails to parse or evaluate; that is a
  separate decision about hook dispatch, unaffected by this change

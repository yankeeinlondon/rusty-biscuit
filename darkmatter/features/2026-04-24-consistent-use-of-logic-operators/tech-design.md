# Technical Design: Consistent Use of Logic Operators

This document defines the technical design for the feature described in
`darkmatter/features/2026-04-24-consistent-use-of-logic-operators/spec.md`.

The change is intentionally narrow: remove bare `|` as an expression-language
operator and make `||` the canonical spelling for fallback in interpolation and
logical OR in conditions. Shell command parsing remains separate and continues
to reject pipe characters in frontmatter shell expressions.

## Context

Darkmatter currently has one expression AST and two parser modes:

- interpolation mode, used by `{{ ... }}` expressions
- condition mode, used by every `when="..."` expression

Those modes share:

- `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/parser.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/ast.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`

Condition evaluation is routed through
`darkmatter/lib/src/markdown/compose/conditions.rs`, which already calls
`parse_condition()` and renders parse failures through `ConditionError`'s
`BlockError` implementation.

Current operator behavior:

| Context | Bare `|` | `||` | `&&` |
| --- | --- | --- | --- |
| interpolation mode | fallback | fallback alias | lexer error |
| condition mode | fallback | logical OR | logical AND |
| frontmatter shell `$(...)` | rejected | rejected | shell token |

Target operator behavior:

| Context | Bare `|` | `||` | `&&` |
| --- | --- | --- | --- |
| interpolation mode | parse error | fallback | lexer error |
| condition mode | parse error | logical OR | logical AND |
| frontmatter shell `$(...)` | rejected | rejected | shell token |

## Spec Interpretation

The spec's goals say `||` remains the canonical fallback operator for
interpolation while also remaining logical OR in condition mode. The open
question note says "`||` should always mean logical OR"; this design treats that
as applying to condition mode only because the feature's operator table and
migration examples preserve interpolation fallback via `||`.

Under this design:

- `{{ name || "friend" }}` evaluates as fallback.
- `when="a || b"` evaluates as logical OR.
- `when="env.AGENT || env.DEFAULT_AGENT"` is logical OR, not fallback.
- `when="(env.AGENT || env.DEFAULT_AGENT) == 'claude'"` is therefore not a
  semantic replacement for the old fallback expression unless the condition
  evaluator intentionally preserves a value-returning OR. This is the one
  behavior decision that should be confirmed before implementation.

The recommended implementation below preserves today's condition-mode `||`
semantics as logical OR and removes bare fallback from conditions. If product
intent is instead to make condition-mode `||` a value-returning fallback/OR
hybrid, only the condition evaluator changes; the lexer and parser changes stay
the same.

## Goals

1. Reject bare `|` in interpolation expressions with an actionable error.
2. Reject bare `|` in condition expressions with an actionable error.
3. Keep `||` as interpolation fallback.
4. Keep `||` as condition logical OR.
5. Keep `&&` as condition logical AND.
6. Leave frontmatter shell tokenization unchanged.
7. Update active docs, rustdoc examples, tests, and the Darkmatter skill.

## Non-Goals

1. Add `??`, `and`, `or`, or any other new operator.
2. Change truthiness, comparison, ternary, function-call, or variable lookup
   behavior.
3. Rewrite historical feature documents under `darkmatter/features/_completed/`.
4. Add a migration subcommand.
5. Change `::toc-linking` fallback chains, which are pipe-separated directive
   syntax rather than interpolation or condition expressions.

## Design Overview

```mermaid
flowchart TD
    A[Expression source] --> B{Source kind}
    B -->|{{ ... }}| C[Lexer: Interpolation mode]
    B -->|when attribute| D[Lexer: Condition mode]
    C --> E[Parser: interpolation ladder]
    D --> F[Parser: condition ladder]
    E --> G[Expr AST]
    F --> G
    G --> H{Evaluator}
    H -->|interpolation| I[Rendered value]
    H -->|condition| J[Truthy bool]

    C -. bare pipe .-> K[ParseError with use || hint]
    D -. bare pipe .-> L[ParseError with use || hint]
```

The safest implementation is to keep the existing parse-mode split and change
the tokenization contract:

- single `|` becomes an error in both modes
- `||` produces a fallback token in interpolation mode
- `||` produces a logical-OR token in condition mode
- `&&` remains condition-only

The AST can remain unchanged. `Expr::Fallback` still represents interpolation
fallback and is now produced only by `||` in interpolation mode unless the
condition evaluator deliberately adopts value-returning OR later.

## Lexer Changes

Update `Lexer::next_token()` in
`darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`.

### Token Contract

Recommended behavior for the `|` branch:

```rust
match ch {
    '|' => {
        self.advance();
        if self.current_char() == Some('|') {
            self.advance();
            match self.mode {
                ParseMode::Interpolation => Ok(Token::Pipe),
                ParseMode::Condition => Ok(Token::OrOr),
            }
        } else {
            Err(LexerError::new(
                match self.mode {
                    ParseMode::Interpolation => {
                        "Unexpected '|'. Use '||' for fallback."
                    }
                    ParseMode::Condition => {
                        "Unexpected '|'. Use '||' for logical OR."
                    }
                },
                start_pos,
            ))
        }
    }
    // ...
}
```

`Token::Pipe` can stay named as-is for a minimal diff, but its rustdoc should be
updated from "The pipe operator `|`" to "The `||` fallback operator in
interpolation mode." A larger cleanup could rename it to `Fallback`, but that is
not required for the feature.

### Parse Mode Documentation

Update `ParseMode` docs to remove all legacy language:

- interpolation mode: `||` maps to `Token::Pipe`; bare `|` and `&&` are invalid
- condition mode: `||` maps to `Token::OrOr`; `&&` maps to `Token::AndAnd`;
  bare `|` is invalid

## Parser Changes

The parser structure can mostly stay as-is.

### Interpolation Mode

`parse()` continues to call `Parser::new(input)?.parse()`.

`parse_fallback()` can keep the same function name and consume `Token::Pipe`,
but its grammar comments should say:

```text
fallback = comparison ("||" comparison)*
```

Since `Token::Pipe` is now only emitted for `||` in interpolation mode, no parser
logic change is needed to remove bare `|`.

### Condition Mode

`parse_condition()` continues to call
`Parser::with_mode(input, ParseMode::Condition)?.parse()`.

The condition ladder remains:

```text
expression  = ternary
ternary     = logical_or ("?" logical_or ":" logical_or)?
logical_or  = logical_and ("||" logical_and)*
logical_and = comparison ("&&" comparison)*
comparison  = unary (comp_op unary)?
unary       = "!" unary | primary
primary     = literal | variable | function_call | "(" expression ")"
```

The meaningful change is that condition mode no longer has a fallback-precedence
level. The current `parse_logical_and()` calls `parse_fallback()`; update it to
call `parse_comparison()` directly.

That makes `when="a | b"` fail in the lexer and prevents condition-mode
`Expr::Fallback` from being produced from source syntax.

### Parentheses and Ternary

No special handling is required. Parenthesized expressions call
`parse_expression()` and therefore preserve the current mode-specific grammar.
Ternary branches should continue to parse through `parse_ternary_branch()`.

## Evaluator Changes

### Interpolation Evaluator

`darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs` does not need a
semantic change. `Expr::Fallback` should continue to:

1. evaluate the primary expression
2. return it when truthy
3. evaluate and return the fallback when the primary is falsy

Update examples and tests from `|` to `||`.

### Condition Evaluator

`darkmatter/lib/src/markdown/compose/conditions.rs` can keep its existing
`Expr::FunctionCall { name: "Or" | "And", ... }` evaluation path.

Recommended behavior:

- `||` stays boolean OR with short-circuiting.
- `&&` stays boolean AND with short-circuiting.
- `Expr::Fallback` support can remain in the evaluator for defensive
  compatibility with manually constructed ASTs or nested code paths, but no
  condition source syntax should produce it after this change.

If the product decision is that `when="env.AGENT || env.DEFAULT_AGENT"` must
behave as value fallback, then condition-mode `||` should lower to
`Expr::Fallback` instead of `Or(...)`, or `Or(...)` should return the first truthy
value instead of a boolean. That is a broader semantic change and should be
called out separately because it affects expressions such as:

```md
::block when="a || b == 'x'"
```

## Error Reporting

Bare-pipe failures should be raised by the lexer so all parse surfaces receive a
consistent error before the parser constructs any AST.

Recommended messages:

| Mode | Message |
| --- | --- |
| interpolation | `Unexpected '|'. Use '||' for fallback.` |
| condition | `Unexpected '|'. Use '||' for logical OR.` |

Those messages flow through:

- interpolation rewrite errors for `{{ ... }}`
- `ConditionError::Parse` for `when="..."`

`ConditionError::Parse` already renders the expression, caret, line, and message
through `BlockError`. Its operator hint should remove any reference to bare
fallback and list:

```text
Operators: &&  ||  !  ==  !=  >  >=  <
```

If interpolation errors do not currently render through a block-style error, do
not broaden this feature to redesign that path. The required improvement is the
lexer message itself.

## Documentation Changes

Update active docs and generated examples only:

- `.claude/skills/darkmatter/SKILL.md`
- `.claude/skills/darkmatter/compose.md`
- `darkmatter/docs/inline/interpolation.md`
- `darkmatter/docs/inline/fm-interpolation.md`
- `darkmatter/docs/topics/boolean-conditional-logic.md`
- rustdoc examples in `interpolation/{lexer,parser,mod,evaluator,ast}.rs`
- compose tests and examples in `darkmatter/lib/src/markdown/compose/mod.rs`

Do not edit historical feature docs under `darkmatter/features/_completed/`.

The boolean-conditional-logic topic should be rewritten to avoid claiming that
condition-mode `|` is fallback. It should also explicitly state that `||` has
mode-specific semantics:

| Surface | `||` meaning |
| --- | --- |
| `{{ ... }}` | fallback, first truthy value wins |
| `when="..."` | logical OR, returns a boolean |

## Test Plan

### Lexer Tests

In `interpolation/lexer.rs`:

- interpolation mode tokenizes `a || b` as `Variable("a"), Pipe, Variable("b")`
- interpolation mode rejects `a | b`
- interpolation mode still rejects `a && b`
- condition mode tokenizes `a || b` as `Variable("a"), OrOr, Variable("b")`
- condition mode tokenizes `a && b` as `Variable("a"), AndAnd, Variable("b")`
- condition mode rejects `a | b`
- string literals containing pipes still work, for example `"a | b"` and
  `"a || b"`

### Parser Tests

In `interpolation/parser.rs`:

- `parse(r#"foo || "default""#)` returns `Expr::Fallback`
- `parse(r#"foo | "default""#)` returns `ParseError`
- chained interpolation fallback uses `||`
- `parse_condition("a || b")` returns lowered `Or(...)`
- `parse_condition("a && b")` returns lowered `And(...)`
- `parse_condition("a | b")` returns `ParseError`
- `parse_condition("(a || b) && c")` preserves precedence and grouping
- `parse_condition("a || b && c")` parses as `Or(a, And(b, c))`

### Evaluator and Compose Tests

In `interpolation/evaluator.rs` and `compose/mod.rs`:

- `{{ missing || "default" }}` renders `default`
- `{{ primary || "default" }}` renders `primary`
- `{{ missing || backup || "default" }}` uses the first truthy value
- `{{ missing | "default" }}` is not rewritten and produces the new parse error
- `::block when="a || b"` keeps logical OR behavior
- `::block when="a && b"` keeps logical AND behavior
- `::block when="a | b"` fails with `ConditionError::Parse`
- `::file child.md when="enabled && !skip"` still works

### Shell Regression Tests

No parser changes should touch shell tokenization, but keep or add regression
coverage that frontmatter shell expressions reject pipes as they do today:

- `key: "$(echo a | cat)"` remains rejected
- `key: "$(false || echo fallback)"` remains rejected by the shell tokenizer

Body `::shell` command behavior should not be changed by this feature.

## Migration and Audit

Use `rg` to audit active docs and source examples:

```sh
rg -n '\{\{[^}\n]*\|[^|}]' darkmatter .claude/skills/darkmatter
rg -n 'when="[^"]*\|[^|"]*"' darkmatter .claude/skills/darkmatter
rg -n 'fallback.*\\\||\|.*fallback' darkmatter/docs .claude/skills/darkmatter
```

Manual review is required because Markdown tables escape pipe characters and
many results refer to unrelated concepts such as terminal fallback rendering,
TOC-linking fallback chains, or Rust closures.

Migrate expression examples as follows:

| Before | After |
| --- | --- |
| `{{ name | "friend" }}` | `{{ name || "friend" }}` |
| `{{ env.EDITOR | env.VISUAL | "vi" }}` | `{{ env.EDITOR || env.VISUAL || "vi" }}` |
| `when="a | b"` | no direct equivalent unless value fallback is confirmed |
| `when="a | b || c"` | rewrite with explicit boolean logic or confirm desired semantics |

Condition fallback migrations need care because `||` is logical OR in condition
mode. Prefer preserving the author's intent explicitly:

```md
<!-- old fallback-style value comparison -->
::file ./notes.md when="(env.AGENT | env.DEFAULT_AGENT) == 'claude'"

<!-- safest replacement if DEFAULT_AGENT is only used when AGENT is absent -->
::file ./notes.md when="env.AGENT == 'claude' || (!env.AGENT && env.DEFAULT_AGENT == 'claude')"
```

## Implementation Steps

1. Update lexer tokenization and mode docs.
2. Update parser grammar comments and make condition-mode logical AND consume
   `parse_comparison()` instead of `parse_fallback()`.
3. Update condition error hints.
4. Migrate active tests from bare `|` fallback to `||`.
5. Add rejection tests for bare `|` in both parse modes.
6. Migrate active docs, rustdoc, and skill examples.
7. Run focused tests for Darkmatter compose and interpolation.

Suggested verification commands:

```sh
cargo test -p darkmatter interpolation::
cargo test -p darkmatter markdown::compose::conditions
cargo test -p darkmatter markdown::compose::tests::infix_logic_conditions
```

Adjust the exact filters to match the crate's test module paths if Cargo reports
that a filter matched no tests.

## Risks

The main risk is condition fallback migration. The old syntax allowed value
fallback in `when=` expressions, while the target syntax reserves `||` for
logical OR. Any docs or tests that compare a fallback result to a string need a
semantic rewrite, not a mechanical replacement.

The second risk is ambiguous naming in the implementation. Keeping
`Token::Pipe` for a token emitted by `||` is a small internal inconsistency. It
is acceptable for a narrow change, but the docs and comments must be updated so
future work does not reintroduce bare-pipe assumptions.

The third risk is accidental edits to unrelated pipe syntax. `::toc-linking`
fallback chains, Markdown tables, shell command text, and prose about rendering
fallbacks are outside the scope of this expression-language change.

## Open Decision

Confirm condition-mode fallback semantics before implementation:

- Option A: `when="a || b"` remains logical OR only. This matches the previous
  infix-condition design and the current condition parser.
- Option B: `when="a || b"` becomes value-returning fallback/OR. This makes the
  spec's fallback migration examples mechanical but changes existing logical OR
  evaluation semantics.

This design recommends Option A because it preserves the existing condition
operator contract and limits the feature to removing bare `|`.

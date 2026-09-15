# Stage 3 — Grammar

The parser is a hand-written recursive descent over the token stream, one
function per precedence level. It produces a `SpannedExpr`; the span-erased
`Expr` the evaluator consumes is derived from it by `SpannedExpr::erase`, so
there is one grammar and the two AST forms can never disagree.

Implemented in
[`expression/parser.rs`](../../../lib/src/markdown/compose/expression/parser.rs);
the AST lives in
[`expression/ast.rs`](../../../lib/src/markdown/compose/expression/ast.rs).

## The ladder

Each rule delegates to the next-tighter one, so the call order *is* the
precedence table. Loosest at the top:

```text
expression      := ternary

ternary         := ternary_branch ( "?" ternary ":" ternary )?
ternary_branch  := logical_or                       -- condition mode
                 | fallback                         -- interpolation mode

logical_or      := logical_and ( "||" logical_and )*   -- condition mode
fallback        := logical_and ( "||" logical_and )*   -- interpolation mode

logical_and     := comparison ( "&&" comparison )*
comparison      := additive ( comp_op additive )?
additive        := multiplicative ( ( "+" | "-" ) multiplicative )*
multiplicative  := unary ( ( "*" | "/" | "%" ) unary )*
unary           := "!" unary | "-" unary | postfix
postfix         := primary ( "[" expression "]" | "." IDENT )*
primary         := STRING | NUMBER | BOOL
                 | IDENT | IDENT "(" ( expression ( "," expression )* )? ")"
                 | "(" expression ")"
                 | array_literal | object_literal
```

Binary operators are **left-associative** (`a - b - c` is `(a - b) - c`); the
ternary is **right-associative**, and its branches recurse into `ternary`, so
`a ? b : c ? d : e` is `a ? b : (c ? d : e)` and nested ternaries need no
parentheses.

### Comparison does not chain

`comparison` takes an **optional single** operator, not a repetition. `a < b < c`
is therefore a parse error — "Expected end of expression, found '<'" — rather
than a silently wrong `(a < b) < c`. Use `a < b && b < c`.

## Parse modes

The two modes differ in one rule. Interpolation mode routes a ternary branch
through `fallback`; condition mode routes it through `logical_or`. Both land on
the same shared `logical_and`, so the comparison ladder and everything below it
behave identically.

| | Interpolation mode | Condition mode |
| --- | --- | --- |
| Entry points | `parse`, `parse_spanned` | `parse_condition`, `parse_condition_spanned` |
| `\|\|` | `Fallback` node — first truthy operand wins | lowered to `or(a, b)` |
| `&&` | lowered to `and(a, b)` | lowered to `and(a, b)` |

Note that `&&` binds **tighter** than `||` in both modes, because `logical_and`
sits below both `fallback` and `logical_or`. `a || b && c` is `a || (b && c)`.

## Lowering: the AST has no And/Or nodes

Infix `&&` and (in condition mode) `||` are rewritten at parse time into
`FunctionCall` nodes named `and` and `or`. There is no `SpannedExprKind::And`.
Anything that walks the AST — the evaluator, DMLS diagnostics, context-variable
collection — sees a two-argument function call.

One consequence worth knowing when mapping spans: a lowered call's span starts
at its **left operand**, not at a name token, because no name token exists in the
source. Code that resolves "is the cursor on a function name" filters these out
by checking that the source text at the name span actually equals the name.

The `Fallback` node is *not* lowered — interpolation `||` stays a distinct AST
variant.

## Postfix chains

`postfix` loops over `[ … ]` and `.` suffixes after a primary.

- **Bracket access** takes a full expression as the index, so
  `items[-1]`, `config[key]`, and `items[n + 1]` all parse. The closing `]` is
  required.
- **Dot access** here only fires on a `Token::Dot`, which the lexer emits solely
  where a dot could not be folded into a variable — after `)`, `]`, or `}`. A
  plain `a.b.c` arrives as one `Variable` token and never reaches this loop. See
  [lexing.md § Dotted paths fold into one token](./lexing.md#dotted-paths-fold-into-one-token).
- A `.` followed by a number produces the dedicated error "Numeric dot access is
  not supported (use bracket indexing for arrays)".

## Literals

### Arrays

`[1, 2, three]`. Elements are full expressions. Trailing commas are a parse
error, deliberately — "trailing commas are not supported".

### Objects

`{ key: value, "quoted-key": computed }`.

- A **bare** key must be a single `Variable` token containing no `.`, starting
  with an ASCII letter or `_`.
- A **quoted** key may be any string, which is the way to write a key the bare
  form cannot express.
- Duplicate keys are a parse error, reported at the offending key's offset.
- Trailing commas are a parse error.

## Function calls

A `Variable` token immediately followed by `(` becomes a `FunctionCall`. The
parser does **not** check the name against the catalog and does not check
arity — an unknown function is a parse success and an evaluation-time failure.
Name resolution is case-insensitive, but that too happens during evaluation.

## Everything must be consumed

After parsing one expression, the parser requires `Token::Eof`. Leftover tokens
produce "Expected end of expression, found 'x'". This is what catches two
expressions jammed together, a chained comparison, and most typo'd numbers.

## Spans

Every node carries a byte range into the expression text. Composite nodes span
from their leftmost token to their rightmost: a binary node covers
`left.start..right.end`, a parenthesized node covers the parentheses themselves,
a function call covers `name(` through `)`.

`ParseError` carries a `position` — the byte offset of the token where parsing
failed — plus a message. Lexer errors are converted into the same shape, so a
caller cannot tell (and does not need to) whether a failure was lexical or
grammatical.

## What the language server sees

DMLS parses with the same `parse_spanned` / `parse_condition_spanned` entry
points, so the editor's AST is byte-for-byte the compose pipeline's AST. It adds
two static checks the runtime does not perform, both over that AST:

| Diagnostic code | Fires on |
| --- | --- |
| `dm.expression.malformed` | a `ParseError`, ranged from the error position to the end of the interpolation |
| `dm.expression.unknown_identifier` | an identifier matching no frontmatter key, schema property, `ctx.*`, `env.*`, or function |

The unknown-identifier check currently inspects only the expression's **root**
identifier — a bare variable, or the base of a member/index chain. An identifier
in an operand position (inside a binary expression, a ternary branch, or a
function argument) is not checked, so a typo there produces no editor
diagnostic. The same limitation applies to the frontmatter-value variant of the
check.

These editor diagnostics are the earliest place an authoring mistake surfaces.
They are not the last: a malformed or unevaluatable expression must also fail
composition on every surface — see
[index.md § Failure handling](./index.md#failure-handling), including the
known deviation in body interpolation.

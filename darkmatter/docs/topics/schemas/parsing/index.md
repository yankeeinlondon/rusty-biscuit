# Expression Parsing

Darkmatter has one expression language and one implementation of it. Every
surface listed in
[Availability across every surface](../darkmatter-expressions.md#availability-across-every-surface)
reaches the same code, so a change to the grammar changes every surface at once.

This section documents *how* an expression becomes an AST. For what the
resulting AST **means** — truthiness, operator semantics, the function catalog,
namespaces — see [Darkmatter Expressions](../darkmatter-expressions.md).

## The three stages

```text
document text
    │
    │   Stage 1 — SCANNING          ExpressionFinder
    │   locate {{ … }} regions in text; skip fenced code;
    │   recognize {{{ … }}} interpolation literals
    ▼
expression text  ("count > 0 ? 'yes' : 'no'")
    │
    │   Stage 2 — LEXING            Lexer / lex_spanned
    │   characters → byte-spanned tokens
    ▼
token stream     [Variable("count"), CompOp(>), NumberLiteral(0), …, Eof]
    │
    │   Stage 3 — GRAMMAR           Parser (recursive descent)
    │   tokens → SpannedExpr, erased to Expr for evaluation
    ▼
AST              → evaluation
```

| Stage | Document | Source |
| --- | --- | --- |
| 1. Scanning | [scanning.md](./scanning.md) | [`expression/lexer.rs`](../../../lib/src/markdown/compose/expression/lexer.rs) (`ExpressionFinder`) |
| 2. Lexing | [lexing.md](./lexing.md) | [`expression/lexer.rs`](../../../lib/src/markdown/compose/expression/lexer.rs) (`Lexer`) |
| 3. Grammar | [grammar.md](./grammar.md) | [`expression/parser.rs`](../../../lib/src/markdown/compose/expression/parser.rs) |

## Not every surface runs every stage

Stage 1 exists only because some surfaces **embed** expressions inside ordinary
text and something has to find them. Surfaces where the expression *is* the
whole string skip straight to Stage 2.

| Surface | Stage 1 | Starts at |
| --- | --- | --- |
| Body interpolation | yes | markdown-aware scan of the body |
| Frontmatter interpolation | yes | plain scan of each string value |
| `when="…"` conditions | no | the attribute value |
| `$()` ternary condition and branches | no | the token in executed position |
| Public condition API, Claudine loop/hook conditions | no | the caller's string |

## Parse modes

The grammar has two modes, selected by the surface, and they differ in exactly
one place: what `||` means. See [grammar.md](./grammar.md#parse-modes).

| Mode | Entry point | `\|\|` | `&&` |
| --- | --- | --- | --- |
| Interpolation | `parse`, `parse_spanned` | fallback (first truthy wins) | logical AND |
| Condition | `parse_condition`, `parse_condition_spanned` | logical OR | logical AND |

## Failure handling

An expression that cannot be parsed or evaluated is an **authoring error** and
must fail composition, on every surface. A document is not more useful for
having a broken placeholder silently survive into its output; it is less useful,
because the mistake now ships.

| Surface | Parse error | Evaluation error |
| --- | --- | --- |
| Frontmatter interpolation | fatal `MarkdownError`; exit 1 | fatal `MarkdownError`; exit 1 |
| `when="…"` conditions | fatal `ConditionError`; exit 1 | fatal `ConditionError`; exit 1 |
| Body interpolation | fatal; exit 1 | fatal; exit 1 |

> **Known deviation.** Body interpolation does not yet honor this rule: a parse
> or evaluation failure there emits a warning on stderr, leaves the `{{ … }}` in
> the output verbatim, and exits 0. That is a defect, not a design — it is why a
> `{{spec-name}}` typo can reach a composed prompt unnoticed. Tracked by
> [dasherized identifiers](../../../features/2026-09-15-dasherized-identifiers/spec.md).

The language server also carries static checks over the same AST, so an author
sees the problem while editing rather than at compose time. Those checks
complement the runtime contract; they do not substitute for it. See
[grammar.md](./grammar.md#what-the-language-server-sees).

## Where a span points

Stage 2 and Stage 3 spans are byte offsets **into the expression text**, not
into the document — Stage 1 is what knows the document offset. A consumer that
needs document coordinates adds the interpolation region's inner start to the
expression-relative span. Spans exclude the whitespace between tokens.

## See Also

- [Darkmatter Expressions](../darkmatter-expressions.md) — semantics, operators, functions
- [Interpolation](../../inline/interpolation.md) — the authoring surface and `{{{ … }}}` literals
- [Frontmatter Interpolation](../../inline/fm-interpolation.md) — pass ordering and dependency resolution

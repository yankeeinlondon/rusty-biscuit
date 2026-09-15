# Stage 2 — Lexing

Lexing turns expression text into a token stream. It is purely lexical: the
lexer never consults frontmatter, context, the filesystem, or the function
catalog, so the same text always produces the same tokens.

Implemented by `Lexer` in
[`expression/lexer.rs`](../../../lib/src/markdown/compose/expression/lexer.rs).

## Two entry points

| Function | Returns | Used by |
| --- | --- | --- |
| `Lexer::tokenize_all()` | `Vec<Token>` | direct callers, tests |
| `lex_spanned(input, mode)` | `Vec<Spanned<Token>>` | the parser, and therefore everything else |

Both always end with `Token::Eof`. Spans are byte ranges into the expression
text and are captured **after** whitespace is skipped, so a token's span covers
the token only.

## Whitespace

Any character matching `char::is_whitespace` separates tokens and is otherwise
discarded. Whitespace is never significant — `a||b` and `a || b` are the same
token stream.

## Identifiers

An identifier starts with a character where `char::is_alphabetic()` is true, or
`_`. It continues with `char::is_alphanumeric()` or `_`.

These are the **Unicode** predicates, not the ASCII ones, so `café` and `größe`
are valid identifiers. Digits may appear after the first character (`foo4`) but
never at the start — which is why a bare `4` is unambiguously a number and never
a property reference.

Characters that are **not** identifier characters therefore end an identifier,
`-` among them. `spec-name` is not one token; it lexes as `spec`, `Minus`,
`name`, and the parser builds a subtraction. This is the single most common
authoring surprise in the language, because kebab-case keys are ordinary in
YAML. Reach such a key with bracket access instead:

```md
{{ doc['spec-name'] }}
```

### Dotted paths fold into one token

`a.b.c` is a **single** `Token::Variable("a.b.c")`, not three tokens joined by
dots. The lexer absorbs a `.` into the identifier only when the character after
it can *start* an identifier.

Two consequences:

- `items.0` does not fold: it lexes as `Variable("items")`, `Dot`,
  `NumberLiteral(0)`, and the parser rejects it with a dedicated
  numeric-dot-access error. Use `items[0]`.
- `Token::Dot` only ever reaches the parser where a dot could not be folded —
  after `)`, `]`, or `}`. Member access on a plain path is already inside the
  variable token.

### `true` and `false`

The lexer reads them as identifiers and then re-classifies them as
`Token::BoolLiteral`. They can never be property references.

## Numbers

`Token::NumberLiteral(f64)`. ASCII digits, optionally followed by a single `.`
and more ASCII digits.

Not supported, and each fails as a parse error rather than silently:

| Form | Why |
| --- | --- |
| `1e3` | no exponent syntax — lexes as `1` then identifier `e3` |
| `.5` | must start with a digit — leading `.` lexes as `Token::Dot` |
| `1_000` | no digit separators — lexes as `1` then identifier `_000` |
| `0xff` | no radix prefixes |

Leading zeros are accepted: `007` is `7`.

Numbers are **always non-negative** at this stage. `-5` is `Minus` followed by
`5`; the parser turns it into unary minus. That is what keeps `5 - 3` from
collapsing into two literals.

## Strings

Single or double quoted; the opening quote determines the closing quote, so
`"it's"` needs no escaping.

| Escape | Produces |
| --- | --- |
| `\n` `\t` `\r` | newline, tab, carriage return |
| `\\` | backslash |
| `\"` in a `"…"`, `\'` in a `'…'` | the quote character |
| anything else, e.g. `\d` | kept verbatim, backslash included |

An unterminated string is a lexer error, as is input that ends immediately after
a backslash.

## Operator tokens

| Input | Token |
| --- | --- |
| `?` `:` `,` `.` | `Question` `Colon` `Comma` `Dot` |
| `(` `)` `[` `]` `{` `}` | `LParen` `RParen` `LBracket` `RBracket` `LBrace` `RBrace` |
| `+` `-` `*` `/` `%` | `Plus` `Minus` `Star` `Slash` `Percent` |
| `==` `!=` `>` `>=` `<` `<=` | `CompOp(…)` |
| `!` | `Bang` (unless followed by `=`) |
| `&&` | `AndAnd` |
| `\|\|` | `Pipe` in interpolation mode, `OrOr` in condition mode |

### Characters that must be doubled

Three characters are only ever valid as a pair, and each has its own error
message rather than a generic one:

- a lone `&` — "Unexpected character: '&'"
- a lone `|` — "Use '||' for fallback" (interpolation) or "Use '||' for logical
  OR" (condition)
- a lone `=` — "Expected '=' after '=' for equality operator"

There is no bitwise operator set and no single-`=` assignment anywhere in the
language, so these are always typos.

Any other unrecognized character produces "Unexpected character: 'x'" at its
byte offset.

## Mode affects exactly one token

`ParseMode::Interpolation` (the default) lexes `||` as `Token::Pipe`;
`ParseMode::Condition` lexes it as `Token::OrOr`. Everything else — including
`&&`, which is logical AND in both modes — is mode-independent. See
[grammar.md § Parse modes](./grammar.md#parse-modes) for what the parser then
does with each.

## Errors

`LexerError` carries a message and the **byte position** in the expression text
where lexing stopped. The parser converts it to a `ParseError` unchanged, so a
lexical failure and a grammatical failure are reported the same way to callers.

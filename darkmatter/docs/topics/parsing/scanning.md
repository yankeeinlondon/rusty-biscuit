# Stage 1 — Scanning

Scanning finds the `{{ … }}` regions inside a body of text and hands their inner
content to [lexing](./lexing.md). It is the only stage that knows about markdown
structure or document byte offsets.

Implemented by `ExpressionFinder` in
[`expression/lexer.rs`](../../../lib/src/markdown/compose/expression/lexer.rs).

## Two entry points

| Constructor | Skips code blocks | Used for |
| --- | --- | --- |
| `ExpressionFinder::new(content)` | yes | document bodies |
| `ExpressionFinder::scan_plain(input)` / `find_all_plain(input)` | no | frontmatter string values, and any string with no markdown structure |

`scan()` is the primary call and returns both expressions and interpolation
literals in one pass; `find_all()` is the convenience wrapper that returns only
expressions.

## What counts as code

Code regions are pre-computed with `pulldown_cmark` (`Options::all()`, offset
iterator) and cover **fenced and indented code blocks** only.

**Inline code spans are deliberately not excluded.** `` `{{ phase }}` ``
interpolates, because templating inside backticks (`` `var_{{ phase }}` ``) is a
common and wanted pattern. The consequence: to show literal `{{ … }}` syntax
inside backticks you need an [interpolation literal](#interpolation-literals),
and to show literal `{{{ … }}}` syntax you need a fenced block.

## Region recognition

The scanner walks the content bytes and, at each position, tests for an
interpolation literal opener before an expression opener.

### Interpolation literals

- A literal opens at **exactly three** consecutive `{`. A fourth `{` disqualifies
  it, and scanning falls through to the `{{` path.
- It closes at the **first subsequent `}}}`**, so literal content can never
  itself contain `}}}`.
- Content is preserved **verbatim** — never trimmed, lexed, parsed, or evaluated
  — and produces no diagnostic.
- An **unclosed** `{{{` is not a literal: the scanner retries the same byte
  position as an ordinary `{{` opener, preserving the older behavior.
- A literal inside a code region is skipped like any other region.

Authoring rules and examples live in
[Interpolation § Interpolation Literals](../../inline/interpolation.md#interpolation-literals).

### Expressions

- An expression opens at `{{` and closes at the matching `}}`.
- Matching is **brace-depth counted**, not first-match: a nested `{{` increments
  the depth and a `}}` decrements it. So `{{ a {{ b }} }}` is captured as **one**
  region whose inner text is `a {{ b }}` — which then fails to parse and fails
  composition. Nesting is not a feature; the depth counter exists so a stray
  inner `}}` does not truncate the region at the wrong place.
- Inner text is **trimmed** before it reaches the lexer, so `{{name}}` and
  `{{  name  }}` are identical.
- **Empty or whitespace-only** content (`{{}}`, `{{  }}`) yields no expression at
  all — silently, with no diagnostic.
- An **unclosed** `{{` is skipped; scanning resumes just past the opener.

## What the scanner returns

```rust
pub struct ExpressionLocation {
    pub start: usize,       // byte offset of the first `{`
    pub end: usize,         // byte offset after the last `}`
    pub expression: String, // inner text, trimmed
}
```

`start`/`end` cover the **whole construct including the braces**. Consumers that
want to range a sub-expression inside the region compute
`start + 2 + <expression-relative offset>`; see
[index.md § Where a span points](./index.md#where-a-span-points).

Interpolation literals come back as `InterpolationLiteral` with the same
`start`/`end` convention and untrimmed `content`.

## Ordering and rewriting

Regions are returned in document order. The interpolation rewriter applies
replacements **from the end of the string backward**, so earlier offsets stay
valid as the text changes length. Literal conversion (`{{{ … }}}` → `{{ … }}`)
runs after the final scan pass over a surface, so a literal produced *by* a
replacement value is still converted exactly once.

## Frontmatter has no markdown structure

Frontmatter string values are scanned with the plain entry point — there are no
code fences to respect inside a YAML scalar. Which keys get scanned, in which
pass, and how dependencies between templated keys are ordered is a separate
concern documented in
[Frontmatter Interpolation](../../inline/fm-interpolation.md).

## Language-server use

DMLS scans with the same `ExpressionFinder` so the editor and `md compose` agree
on what is an expression. It additionally filters by a `body_base` offset to
separate body literals from frontmatter literals, since the two get different
diagnostics and different source mappings.

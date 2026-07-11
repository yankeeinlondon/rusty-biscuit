---
status: draft — awaiting review
created: 2026-07-10
review_iterations: 2
area: darkmatter
packages:
  - darkmatter
  - dmls
---

# Interpolation Literals (`{{{ ... }}}`)

## Status

This specification defines the functional contract for the interpolation
literal — a triple-brace span that renders as literal `{{ ... }}` text and
whose contents are never scanned, parsed, or evaluated as an interpolation
expression. It covers scanner recognition, compose semantics, DMLS behavior,
and the guarantees every consumer of the shared expression scanner inherits.

## Purpose

Documentation frequently needs to *show* the interpolation syntax rather than
*use* it — prose such as `` `{{ … }}` `` describing template behavior. Today
that text is a live scanning target: fenced and indented code blocks are
excluded from expression scanning, but inline code spans are deliberately
included (the `` `var_{{ phase }}` `` templating pattern is supported), so a
documentation-literal `{{ … }}` in inline code or plain prose produces:

- a `dm.expression.malformed` diagnostic from DMLS, and
- a compose parse warning (or a fatal error under `fail_fast`), with the raw
  text leaking through unconverted.

There is currently **no escape mechanism**: the scanner does a raw byte scan
for `{{` with no backslash awareness, so `\{` has no effect.

The interpolation literal closes this gap explicitly. `{{{ content }}}`
composes to the literal text `{{ content }}` and the content is inert on every
scanning surface.

This syntax is backward-compatible: a triple-brace span is malformed under the
current grammar (the scanner captures a stray leading `{` that the lexer
rejects), so no valid existing document can contain one.

## Terminology

- **Interpolation literal** (or **literal**): a source span of the form
  `{{{ content }}}` recognized by the scanner rules below.
- **Opener**: the `{{{` that begins a literal.
- **Closer**: the `}}}` that ends a literal.
- **Content**: the bytes between opener and closer, preserved verbatim
  (never trimmed, never lexed).
- **Conversion**: the compose-time rewrite of a literal to `{{ content }}` —
  the opener becomes `{{`, the closer becomes `}}`, content bytes unchanged.
- **Scanner**: `ExpressionFinder`
  (`darkmatter/lib/src/markdown/compose/expression/lexer.rs`), the single
  shared authority that locates `{{ }}` expressions for compose, DMLS,
  preflight context collection, and remote-reference discovery.

## Recognition Rules (scanner)

Recognition happens at the scanner level, before any lexing, and applies to
both scan modes (`MarkdownAware` and `Plain` / `find_all_plain`).

1. **Opener — exactly three braces, maximal munch.** A literal opens at a run
   of exactly three consecutive `{` characters. The scanner MUST check for
   `{{{` before `{{`. A run of four or more `{` is **not** a literal opener;
   it falls through to the pre-existing `{{` scanning behavior byte-for-byte
   (today: nested-depth scanning that typically yields a malformed
   expression). This keeps runs longer than three deterministic without
   inventing semantics for them.
2. **Closer — first `}}}`.** A literal ends at the first occurrence of `}}}`
   after the opener. Consequences, both intended:
   - content cannot itself contain `}}}` (the standard raw-syntax regress;
     fenced code blocks remain the out), and
   - in a run of four or more `}`, the first three close the literal and the
     remainder is plain text.
   The closer search mirrors the existing `}}` search: raw text, to end of
   input, no code-region awareness after the opener.
3. **Unclosed opener — legacy fallback.** A `{{{` with no subsequent `}}}` is
   not a literal. The scanner MUST fall back to the pre-existing behavior at
   the same position (i.e., `{{` matches at the first byte and the third `{`
   is part of the captured expression, malformed today). Rationale: an
   unclosed literal is a typo; silently promoting its tail to a live
   expression would change behavior invisibly, whereas the legacy malformed
   diagnostic points at exactly the suspicious span.
4. **Code regions unchanged.** The opener is subject to the same code-region
   exclusion as `{{`: inside fenced and indented code blocks, `{{{ ... }}}`
   is plain text — never recognized, never converted. Inline code spans are
   scanned as they are today, which is precisely where literals are most
   useful.
5. **Nesting and adjacency.** Literals do not nest. `{{ a }}{{{ b }}}` is one
   expression followed by one literal. `{{{ {{ x }} }}}` is a single literal
   whose content is ` {{ x }} ` (rule 2 governs where it closes); the inner
   `{{ x }}` is never evaluated.
6. **Empty content is allowed.** `{{{}}}` and `{{{ }}}` are literals
   converting to `{{}}` and `{{ }}` respectively. (Contrast: the expression
   scanner drops empty `{{ }}` spans; literals never require parseable
   content.)

The scanner MUST expose literal spans as a distinct product (alongside
`ExpressionLocation`) so passive consumers (DMLS hover, code actions) can see
them without re-implementing recognition.

## Compose Semantics

### Inertness during scanning

On every pass of every interpolation surface, a literal is skipped whole: its
content is never tokenized, parsed, or evaluated, and it never produces a
warning or error regardless of what it contains. `{{{ > invalid … }}}` is as
valid as `{{{ name }}}`.

### Conversion point

Conversion (`{{{ content }}}` → `{{ content }}`) MUST happen exactly once per
surface, **after the final scanning pass over that surface**. This ordering is
load-bearing in two places:

- **Body rescan loop.** `interpolate_text` rescans its own output (up to the
  depth limit) so replacement values can introduce new expressions. Literals
  MUST survive every rescan iteration untouched and convert only after the
  loop terminates — converting earlier would make the emitted `{{ }}` live on
  the next iteration.
- **Frontmatter two-pass interpolation.** Frontmatter values interpolate in
  two passes bracketing shell expansion. A literal in a frontmatter value
  MUST survive pass 1 (including the deferred-key path) and convert only
  after pass 2. When shell expansion is disabled and pass 2 does not run,
  conversion happens at the end of the single pass that did run.

A `{{{ ... }}}` introduced *by a replacement value* during the body rescan
loop is treated the same as an authored one: skipped while scanning continues,
converted at the end.

### Frontmatter value semantics

A literal is always **text**, never a typed whole-value expression. A value
whose trimmed content is exactly one literal (`key: "{{{ x }}}"`) takes the
string path and resolves to the string `{{ x }}` — the whole-value
typed-evaluation contract applies only to real expressions.

### Surfaces in scope

Literal recognition and conversion apply to every surface the shared scanner
serves for `{{ }}` interpolation:

- body interpolation (both `MarkdownAware` and the `interpolate_code_blocks`
  plain mode),
- frontmatter value interpolation (both passes), and
- any future surface that scans via `ExpressionFinder`.

### Surfaces out of scope

- Frontmatter `$( ... )` shell values, including their ternary branches. The
  `$()` grammar is separate; this feature defines no escape for it.
- Fenced/indented code blocks (already inert; rule 4).
- The `replace:` text-replacement map (operates on literal strings; no
  interaction).

### Inert-consumer guarantees

Every consumer of the shared scanner MUST treat literal content as inert:

- **Demand-driven context capture** (preflight collection): `{{{ ctx.hardware
  }}}` MUST NOT trigger the hardware probe, nor any other context-group
  capture.
- **Remote-reference discovery**: a URL or `file(...)` form inside a literal
  MUST NOT drive remote prefetch or fetch-policy evaluation.
- **DMLS substrate indexing**: a literal MUST NOT materialize
  `NodeKind::Interpolation` nodes or `uses_variable` edges.

## DMLS Behavior

- **Diagnostics.** No `dm.expression.*` diagnostic is ever emitted for a
  literal's span or content. (An *unclosed* `{{{` falls back to legacy
  scanning per rule 3, so it keeps whatever diagnostic the fallback produces —
  intentionally.)
- **Hover** (required): hovering a literal renders a short block identifying
  it as an interpolation literal, showing the composed output (`{{ content
  }}`), and noting that the content is not interpolated.
- **Code action** (SHOULD; deferrable to a follow-up): on a
  `dm.expression.malformed` diagnostic, offer "wrap in interpolation literal"
  — rewriting the offending `{{ ... }}` to `{{{ ... }}}` via the existing
  diagnostic-driven code-action machinery.
- **Formatting/rename/completion**: no participation. Literals are opaque
  text to every other provider.

## Precedent Note

Handlebars/Mustache assign `{{{ x }}}` the *opposite* meaning (live,
HTML-unescaped interpolation). A Darkmatter document that quotes Handlebars
triple-brace syntax verbatim in prose or inline code will compose it down to
`{{ x }}`. This collision is accepted; the workaround for documenting
Handlebars is a fenced code block (rule 4).

## Rendering Without Compose

A document rendered without composing (e.g. `md render` on raw source, or a
DMLS-diagnosed design doc that never composes) shows `{{{ ... }}}` verbatim,
including inside inline code spans. This is the same class of wart any
escape syntax carries and is accepted: the source form reads as intentional
bracketing.

## Documentation Updates

- **`darkmatter/docs/inline/interpolation.md`** (required, same change set):
  add an **Interpolation Literals** section documenting the `{{{ ... }}}`
  syntax, the first-`}}}` termination rule, the unclosed fallback, and the
  fenced-code-block alternative; update the **Implementation** section's
  scanner description (which currently documents only code-block exclusion)
  to mention literal recognition.
- `darkmatter/docs/topics/darkmatter-expressions.md`: add the literal to the
  expression-surface documentation.
- `.claude/skills/darkmatter/compose.md` and the skill's interpolation
  coverage: mention the literal; regenerate the skill file's `hash:`
  frontmatter with `md hash <file>` after editing.

## Acceptance Criteria

1. `{{{ name }}}` in body prose composes to the literal text `{{ name }}`
   with `name` unevaluated, zero warnings, zero replacements counted for it.
2. `` `{{{ … }}}` `` in an inline code span composes to `` `{{ … }}` `` and
   produces no DMLS diagnostic — the motivating case.
3. `` `var_{{ phase }}` `` (no literal) still interpolates — the existing
   inline-code templating contract is unchanged.
4. Tight form `{{{x}}}` → `{{x}}`; empty forms `{{{}}}` → `{{}}` and
   `{{{ }}}` → `{{ }}`.
5. Adjacency: `{{ a }}{{{ b }}}` evaluates `a` and emits `{{ b }}` literally.
6. Four-brace opener `{{{{ x }}}}` is not a literal and reproduces today's
   behavior byte-for-byte.
7. Unclosed `{{{ x }}` reproduces today's behavior byte-for-byte (legacy
   malformed diagnostic; no silent promotion of `x` to a live expression).
8. Literal containing a valid expression `{{{ {{ x }} }}}` emits
   `{{ {{ x }} }}` with `x` unevaluated.
9. A literal inside a fenced code block is untouched source text after
   compose (no conversion).
10. Frontmatter: `key: "{{{ x }}}"` resolves to the string `{{ x }}`; the
    literal survives both interpolation passes (verified with a document
    using frontmatter shell expansion) and is not treated as a whole-value
    expression.
11. Body rescan loop: a replacement value that introduces `{{{ y }}}`
    composes to literal `{{ y }}` (not an evaluation of `y`).
12. `{{{ ctx.hardware }}}` does not trigger hardware context capture; a
    remote URL inside a literal does not appear in remote-discovery output.
13. DMLS: no `dm.expression.*` diagnostic on a literal; hover renders the
    literal block; no `uses_variable` edge or `Interpolation` node is
    indexed for it.
14. `fail_fast` compose over a document whose only `{{ }}`-shaped content is
    literals succeeds.

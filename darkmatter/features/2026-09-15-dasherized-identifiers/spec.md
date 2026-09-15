---
created: 2026-09-15
status: draft
reviewed: false
---

# Dasherized Identifiers

Darkmatter's expression grammar cannot reference a kebab-case frontmatter key.
`{{{ spec-name }}}` parses as `spec` minus `name`, because `-` is only ever the
subtraction operator and identifiers are `[alphabetic|_][alphanumeric|_]*`
([`lexer.rs:952-960`](../../lib/src/markdown/compose/expression/lexer.rs)).

Kebab keys are ordinary YAML and ordinary frontmatter. The grammar must support
them.

This spec also fixes the reason the problem went unnoticed for so long: a body
interpolation that fails to parse or evaluate currently warns and passes the
`{{{ … }}}` through to the output instead of failing composition.

The current parsing design — the three stages, the identifier rules, the
grammar ladder, and the failure contract — is documented under
[`docs/topics/parsing/`](../../docs/topics/parsing/index.md). That documentation
is the "before" picture this spec changes.

## Status

Draft. Not yet reviewed.

## Motivation

### The incident

`prompts/_reviews/feature-review.md` declared `spec_name` in its schema and
referenced it in the body as `{{{spec-name}}}`. Composition emitted:

```text
warning: [interpolation] failed to evaluate 'spec - name': Subtraction requires numeric operands
```

and shipped the literal text ``the `{{{spec-name}}}` spec`` into the prompt that
was handed to a review agent. Three separate failures lined up:

1. The grammar could not express what the author meant.
2. The runtime downgraded the failure to a warning and emitted the broken text.
3. The language server said nothing while the file was being edited.

All three are in scope here.

### Kebab keys are common, and not always ours to rename

A repo-wide scan of `{{{ … }}}` bodies in `prompts/`, `claudine/`, `darkmatter/`,
and `.claude/` found:

- **Exactly one** genuine subtraction: `{{{_loop_count - 1}}}` — spaced.
- Every other unspaced `-` is either inside a string literal (`'review-' +
  iteration`, `"x-"`, `'monorepo-root'`), which the lexer never sees, or is an
  author already writing a kebab variable and expecting it to resolve:
  `{{{absolute-filepath}}}`, `{{{review-file}}}`, `{{{fromJSON(inputs.l2-environments)}}}`.

Kebab frontmatter keys in the repo include `depends-on`, `supersedes-revision`,
`document-schema`, and — in `.claude/commands/` — `argument-hint` and
`allowed-tools`, which belong to Claude Code's frontmatter schema and cannot be
renamed to suit Darkmatter.

### An escape hatch exists but nothing points at it

`{{{ doc['spec-name'] }}}` resolves correctly today (verified). It is undiscoverable:
neither the warning text, the docs prior to this change, nor DMLS mentions it,
and the two spellings an author reaches for first — `{{{spec-name}}}` and
`{{{doc.spec-name}}}` — both fail the same silent way.

## Goals & Non-Goals

**Goals**

- A kebab-case frontmatter key is referenceable by its own name in every
  expression surface.
- A malformed or unevaluatable expression fails composition everywhere.
- An identifier that names nothing is diagnosed by DMLS wherever it appears in
  an expression, not only at the root.
- A reference to a name the document never declared is surfaced as a **warning**,
  not an error, and never silently.
- Each problem is reported exactly once per run.

**Non-Goals**

- Whitespace-significant operators beyond the single rule below.
- Changing how unresolved-but-well-formed variables **render**. They stay an
  empty string and composition still succeeds; Requirement 4 adds a warning
  beside that output, not a different output.
- Renaming or normalizing existing kebab keys. They are valid input.
- Any change to evaluation semantics, the function catalog, or namespaces.

## Requirement 1 — `-` may continue an identifier

### The rule

> A `-` continues the current identifier if and only if the lexer is already
> **mid-identifier** and the next character continues an identifier
> (`char::is_alphanumeric()` or `_`). Otherwise `-` is an operator.

Stated for authors: **binary minus requires whitespace before it**, unless the
left operand ends in `)`, `]`, or a digit that is a number literal.

The decision lives inside `read_variable`
([`lexer.rs:875-919`](../../lib/src/markdown/compose/expression/lexer.rs)), not
in a character class. That placement is what makes the numeric case fall out for
free: `foo4` is consumed as one identifier token, so the scanner is
mid-identifier at a following `-` and joins; a bare `4` is consumed by
`read_number`, so the scanner is *not* mid-identifier and the `-` is an
operator. No lookbehind and no token-history inspection is required.

### Behavior

| Expression | Today | After |
| --- | --- | --- |
| `spec-name`, `depends-on`, `argument-hint` | `spec - name` | identifier |
| `l2-environments`, `phase-2`, `level-3` | subtraction | identifier |
| `doc.spec-name` | `doc.spec - name` | identifier path |
| `a - b`, `a -b`, `a- b` | subtraction | subtraction |
| `4-2` | subtraction | subtraction |
| `f(x)-1`, `arr[0]-1`, `(a)-1` | subtraction | subtraction |
| `-5`, `a * -1` | unary minus | unary minus |
| `iteration-1` | subtraction | **identifier** (breaking) |
| `foo--bar` | `foo - (-bar)` | `foo - (-bar)` (second `-` cannot continue) |

`iteration-1` is the only breaking case. The corpus contains no unspaced
subtraction, so nothing in this repository changes meaning. The correct spelling
becomes `{{{ iteration - 1 }}}`.

### Both loops, not just the first

`read_variable` scans a leading identifier and then dotted path segments in a
second loop. Both must accept the joined `-`, or `doc.spec-name` stays broken
while `spec-name` works.

### The bracket form remains

`doc['spec-name']` keeps working and stays the way to reach a key the identifier
grammar cannot spell — a key containing `.`, `--`, a leading digit, or a
trailing `-`.

## Requirement 2 — Invalid expressions fail composition

An expression that cannot be parsed or evaluated is an authoring error on every
surface. Body interpolation must stop degrading it to a warning.

| Surface | Parse error | Evaluation error | Today |
| --- | --- | --- | --- |
| Frontmatter interpolation | fatal, exit 1 | fatal, exit 1 | correct |
| `when="…"` conditions | fatal, exit 1 | fatal, exit 1 | correct |
| Body interpolation | fatal, exit 1 | fatal, exit 1 | **warns, emits `{{{ … }}}` verbatim, exit 0** |

The error must carry the source line number and render through the same rich
error surface the other two use, so the three are indistinguishable to a caller.

This requirement is independent of Requirement 1 and could ship first. It is in
this spec because the two together are what turn the motivating incident from
"silently wrong output" into "a build that stops."

Reporting obligations for this requirement are governed by
[Requirement 5](#requirement-5--one-diagnostic-per-issue).

## Requirement 3 — DMLS checks every identifier, not just the root

`dm.expression.unknown_identifier` already exists and already runs over body
interpolations and frontmatter expression values. It only ever inspects the
expression's **root** identifier, via `root_identifier`
([`overlay/expressions.rs:121-129`](../../dmls/src/overlay/expressions.rs)),
which returns `Some` for a bare `Variable` and for the base of a
member/index chain, and `None` for everything else.

`{{{spec-name}}}` parses to `Binary { Variable("spec"), Sub, Variable("name") }`,
so `root_identifier` returns `None` and the check silently no-ops. Every
identifier in an operand position is unchecked: `{{{a - b}}}`, `{{{x ? y : z}}}`,
`{{{f(arg)}}}`, `{{{a || b}}}`.

The fix is to walk the AST and diagnose every unknown `Variable` node at its own
span. The evaluator already contains exactly this traversal for the `ctx.*`
namespace — `walk_context_variables` / `collect_context_warnings` in
[`interpolation/evaluator.rs`](../../lib/src/markdown/compose/interpolation/evaluator.rs)
— which recurses Binary, Ternary, Fallback, Index, and MemberAccess and reports
`unknown context variable 'ctx.foo'` with a did-you-mean. The new walk should
share that shape.

This matters more after Requirement 1, not less: `{{{iteration-1}}}` becomes a
well-formed reference to a key that does not exist. Requirement 4 catches it at
compose time; this requirement is what catches it while the author is still
typing, and what catches the same class of typo in an operand position
regardless of dashes.

### Kebab-specific diagnostic

When an expression is `Binary { Variable(a), Sub, Variable(b) }` **and** the
document's frontmatter has a key literally named `a-b`, that is a mis-referenced
key rather than arithmetic. DMLS has both halves already
(`is_unknown_identifier` at [`dsl.rs:891`](../../dmls/src/providers/dsl.rs) uses
`ast.entry_by_dotted`). It should say so and offer the quick-fix.

After Requirement 1 this case mostly disappears for *new* documents, but it
remains the right diagnostic for a document pinned to older behavior and for
`a-b` spellings the new rule still treats as subtraction (`foo--bar`, `a- b`).

## Requirement 4 — An unresolved undeclared identifier warns

A well-formed identifier that resolves to nothing is **not** an error. Its type
is `any` by default, and `null`/undefined is a valid inhabitant of `any`, so
composition proceeds and it renders as an empty string exactly as it does today.
But in most documents it is a typo, so it must not be silent either.

The deciding question is whether the document **declared** the name. A
`$schema` declaration is the document stating that the name is expected; without
one, the reference is unvouched-for and probably a typo.

### Worked examples

In every row, `iteration-1` has no value in frontmatter and none supplied by the
caller.

| Expression | `$schema` | Result | Why |
| --- | --- | --- | --- |
| `{{{ iteration-1 }}}` | not declared | **warning**, renders empty, exit 0 | nothing vouches for the name |
| `{{{ iteration-1 \|\| "fallback" }}}` | not declared | silent, renders `fallback` | the author handled the absence |
| `{{{ iteration-1 }}}` | `iteration-1: number` | silent, renders empty | declared and optional, so `null` is in the type |
| `{{{ iteration-1 }}}` | `iteration-1: number(required)` | **fatal**, exit 1 | not this requirement — see below |

### A required property is already handled

A required property left unset fails schema validation before interpolation runs
at all:

```text
missing iteration-1: required but not provided
```

Requirement 4 must **not** add a warning for that case. The property is
declared, so the schema gate already owns it, and a second message about the
same missing value would violate
[Requirement 5](#requirement-5--one-diagnostic-per-issue). The `$schema` gate in
Requirement 4 therefore keys on *declared or not*, and never on required-ness —
required-ness is spent before the evaluator sees the document.

### Current behavior, for contrast

Verified today: a declared-optional-unset reference and an undeclared-unset
reference both render empty and exit 0 with no diagnostic whatsoever. Requirement
4 changes exactly one of those two — the undeclared one.

This is the missing runtime half of a rule DMLS already implements. Its
`is_unknown_identifier` ([`dsl.rs:880-903`](../../dmls/src/providers/dsl.rs))
already treats a schema-declared property as known "even when the document
leaves it unset", and already exempts `ctx.*` / `env.*` / `doc.*` roots and
function names. Requirement 4 gives the runtime the same authority so the editor
and `md compose` agree on what counts as a mistake.

### Where the runtime is better informed than the editor

DMLS declines to fire at all on a frontmatter-less document, because any bare
identifier there could be a `--set` value it cannot see. The runtime has no such
blind spot: it warns only after resolution has actually produced nothing, so a
value supplied by `--set`, inherited parent state, a caller input layer, or the
`ctx.*` fallback never warns regardless of what the frontmatter declares.

### Explicitly-handled absence must stay silent

`{{{ color || "unknown" }}}` and `{{{ color ? color : "none" }}}` are the
documented idioms for a value that may be absent
([`inline/interpolation.md`](../../docs/inline/interpolation.md)). Warning on
them would punish the author for handling the case correctly and would make the
warning worthless through noise.

The warning is suppressed when the unresolved identifier is the **primary of a
`Fallback`** node or the **condition of a `Ternary`**. Both are explicit
"this may be absent" constructs. An identifier anywhere else — including the
right-hand side of a fallback and either branch of a ternary — still warns.

| Expression | Unresolved `x` warns? |
| --- | --- |
| `{{{ x }}}` | yes |
| `{{{ x \|\| "d" }}}` | no — primary of a fallback |
| `{{{ a \|\| x }}}` | yes — right-hand side |
| `{{{ x ? a : b }}}` | no — ternary condition |
| `{{{ a ? x : b }}}` | yes — ternary branch |

### Severity alignment

DMLS currently reports `dm.expression.unknown_identifier` at `INFORMATION`
([`dsl.rs:696`](../../dmls/src/providers/dsl.rs),
[`diagnostics/frontmatter.rs:653`](../../dmls/src/diagnostics/frontmatter.rs)).
Since the same condition is a warning at runtime, raise the diagnostic to
`WARNING` so one condition does not carry two severities.

## Requirement 5 — One diagnostic per issue

A compose run reports each problem **exactly once**. Repetition trains authors
to ignore output, and a count that does not match the number of real problems
makes triage guesswork.

This is a requirement of every diagnostic in this spec — the fatal errors of
Requirement 2 and the warnings of Requirement 4 alike.

### Per issue, not per evaluation

One expression that fails is one message, no matter how many times the pipeline
evaluates it. This is currently violated: a single bad body expression emits its
warning **twice**, reproducible in a two-line document with no proxy and no
schema. The cause has not been identified — it is *not* the proxy/redirect path,
which was an early wrong guess, and it survives with and without a `$schema`.
Requirement 2 may remove the visible symptom by turning the warning into a fatal
error, but the underlying double evaluation must be found and fixed rather than
assumed gone.

### Per identifier, not per occurrence

An undeclared, unresolved identifier referenced ten times in one document is one
mistake and produces **one** warning, naming the identifier and the location of
its first occurrence. Two different undeclared identifiers produce two warnings.

### Editor and compose are not duplicates

A DMLS diagnostic and a compose-time message are not the same report twice: they
occur in different processes at different moments, and an author never receives
both for one action. Requirement 3 exists so a mistake is caught while typing;
Requirements 2 and 4 exist so it cannot survive to output if it was not. Neither
substitutes for the other, and neither is suppressed because the other exists.

## Architecture & Touch Points

| # | File | Change |
| --- | --- | --- |
| 1 | [`lexer.rs`](../../lib/src/markdown/compose/expression/lexer.rs) `read_variable` | the joined-`-` rule, in both the leading and dotted-segment loops |
| 2 | [`dmls/src/overlay/expressions.rs:249,619`](../../dmls/src/overlay/expressions.rs) | word-boundary scans (`!(c.is_alphanumeric() \|\| c == '_' \|\| c == '.')`) must accept `-`, or hover/completion/definition truncate `spec-name` at the dash |
| 3 | body interpolation error path | parse and evaluation failures become fatal (Requirement 2) |
| 4 | [`overlay/expressions.rs`](../../dmls/src/overlay/expressions.rs) + [`dsl.rs`](../../dmls/src/providers/dsl.rs) + [`diagnostics/frontmatter.rs`](../../dmls/src/diagnostics/frontmatter.rs) | replace the root-only lookup with a full-AST walk (Requirement 3); both call sites share one helper |
| 5 | interpolation evaluator | the unresolved-identifier warning, gated on the effective `$schema` and on the fallback/ternary suppression (Requirement 4) |
| 6 | [`dsl.rs:696`](../../dmls/src/providers/dsl.rs), [`diagnostics/frontmatter.rs:653`](../../dmls/src/diagnostics/frontmatter.rs) | raise `dm.expression.unknown_identifier` from `INFORMATION` to `WARNING` |
| 7 | body interpolation diagnostic path | find and fix the double emission; dedupe per identifier (Requirement 5) |
| 8 | [`parser.rs:768`](../../lib/src/markdown/compose/expression/parser.rs) `PRECEDENCE_TABLE` | drive-by: it labels Logical AND as `&& (condition mode)`, but `&&` is both modes — the table feeds a user-visible report surface |

DMLS parses through darkmatter's own `parse_spanned` / `parse_condition_spanned`,
so the AST, spans, and both expression diagnostics follow the lexer change for
free. `--set` key validation and schema assignment already accept `-`.

**Side effect, intentional.** Unquoted object-literal keys gain kebab support:
`{ spec-name: 1 }` becomes legal, because the key guard at
[`parser.rs:592`](../../lib/src/markdown/compose/expression/parser.rs) only
requires an alphabetic-or-`_` first character. This is consistent with the rest
of the change; no separate work is needed.

## Compatibility

- **Grammar.** `iteration-1` changes meaning. Zero occurrences repo-wide. Its
  new meaning is an undeclared, unresolved identifier, so Requirement 4 warns at
  compose time and Requirement 3 flags it in the editor. Requirement 1 shipped
  alone would render it as a silent empty string — which is why it should not
  ship alone.
- **Runtime.** Requirement 2 turns previously-passing composes into failures
  wherever a body expression was already broken. That is the point, but it will
  surface latent breakage on first run, including `{{{absolute-filepath}}}` and
  `{{{review-file}}}` — both of which Requirement 1 simultaneously fixes.
- **Persisted output.** None. No AST variant, serialized form, or public
  signature changes.

## Implementation Plan

### Phase 1 — Lexer

The `read_variable` rule, plus the two DMLS word-boundary scans in the same
change so hover and completion do not regress.

### Phase 2 — One diagnostic per issue

Requirement 5's double-emission investigation and fix. Sequenced before the
body-failure change so the duplication is diagnosed while it is still visible as
a warning, rather than being masked by the switch to a fatal error.

### Phase 3 — Fatal body failures

Requirement 2. Independent of Phase 1; sequence either way.

### Phase 4 — Unresolved-identifier warning

Requirement 4 in the evaluator: the `$schema` gate, the fallback/ternary
suppression, and the severity alignment in DMLS. Depends on nothing; benefits
most from landing after Phase 1, so the kebab references it would otherwise
flag are already resolving.

### Phase 5 — DMLS identifier walk

Requirement 3's full-AST walk, shared by the body and frontmatter call sites,
plus the kebab-specific message and quick-fix.

### Phase 6 — Docs

- [`docs/topics/parsing/lexing.md`](../../docs/topics/parsing/lexing.md) — the
  identifier section and the kebab workaround paragraph
- [`docs/topics/parsing/grammar.md`](../../docs/topics/parsing/grammar.md) — the
  language-server section's root-only limitation
- [`docs/topics/parsing/index.md`](../../docs/topics/parsing/index.md) — remove
  the known-deviation callout once Requirement 2 lands
- [`docs/topics/darkmatter-expressions.md`](../../docs/topics/darkmatter-expressions.md)
  — the identifier principle and the unsupported-forms list
- [`docs/inline/interpolation.md`](../../docs/inline/interpolation.md) — the
  empty-string fallback paragraph now carries a warning unless the property is
  schema-declared or the absence is explicitly handled
- `.claude/skills/darkmatter/compose.md` — the parsing pointer

## Testing Requirements

**Lexer (L1).** Every row of the Requirement 1 table as a token-stream
assertion, plus: `-` at end of input; `-` followed by whitespace; `-` followed by
`-`; a kebab segment inside a dotted path; a kebab identifier as an unquoted
object key; `café-name` (the identifier predicates are Unicode).

**Parser (L1).** `{{{ iteration - 1 }}}` still lowers to `Binary`; `4-2`,
`f(x)-1`, `arr[0]-1` still lower to `Binary`; `spec-name` lowers to a single
`Variable` with a span covering the whole name.

**Compose (L2).** A document with a kebab frontmatter key renders it bare, via
`doc.<key>`, and via `doc['<key>']`. Regression: `{{{_loop_count - 1}}}` still
evaluates.

**Failure contract (L2).** For each of the three surfaces, a parse failure and
an evaluation failure each exit non-zero and name the source line. Explicitly
assert the body case exits 1 and does **not** contain the verbatim `{{{ … }}}` in
its output.

**Diagnostic count (L2).** One bad expression produces exactly one message — the
regression test for the current double emission. One undeclared identifier
referenced ten times in a document produces exactly one warning. Two different
undeclared identifiers produce exactly two.

**Unresolved identifiers (L2).** One case per row of the
[worked-examples table](#worked-examples), asserting exit code, rendered output,
and diagnostic count together — including that the `number(required)` row fails
with the schema's own message and **no** additional unresolved-identifier
warning. Every row of the
[suppression table](#explicitly-handled-absence-must-stay-silent), asserting the
branch/right-hand-side positions still warn. And no warning when the value
arrives via `--set`, inherited state, or the `ctx.*` fallback on a document that
does not declare it.

**DMLS (L2).** `dm.expression.unknown_identifier` fires for an unknown
identifier in each operand position: binary operand, ternary branch, function
argument, fallback right-hand side — in both a body interpolation and a
frontmatter expression value. Hover, completion, and go-to-definition resolve on
a kebab identifier at every cursor position within it, including on the dash.

## Resolved Decisions

1. **`-` is not made an identifier character globally.** The join is decided
   inside identifier scanning. A global character class would make `4-2`
   ambiguous.
2. **`-` followed by a digit joins.** `phase-2` is a plausible YAML key; the
   corpus has no unspaced subtraction to protect; and the alternative leaves a
   second class of unreachable keys. The cost is `iteration-1`, which
   Requirement 3 diagnoses.
3. **Whitespace-significant operators are not introduced.** The rule is
   one-directional — it only ever *joins* a `-` into an identifier and never
   changes what an already-spaced expression means.
4. **The documented failure contract wins over current behavior.** The
   pre-existing documentation said invalid expressions fail composition; the
   body-interpolation warning path is the defect. This reverses the repository's
   default drift rule (code wins) by explicit decision.
5. **An unresolved identifier warns rather than failing, and `$schema` is the
   arbiter.** Undeclared means untyped means `any`, and `null` is a valid `any`,
   so failing would be wrong on the type rules alone — but silence is wrong on
   the odds, because most such references are typos. A declared property is the
   document stating that the name is expected, which settles it in the other
   direction. This keeps Requirement 2 (errors are fatal) and Requirement 4
   (absence is a warning) as cleanly separate categories.
6. **A fallback primary and a ternary condition suppress the warning.** Both
   `{{{ x || "d" }}}` and `{{{ x ? a : b }}}` are the documented ways to say "this
   may be absent", so an author who uses either has already handled the case and
   must not be warned for it. Every other position — including a fallback's
   right-hand side — still warns. Approved 2026-09-15.

## Open Questions

1. **Should the suppression set extend past the two approved constructs?** Two
   neighbors read as deliberate absence checks but are not currently exempt:
   `is_empty(x)` / `is_null(x)` arguments, and a bare identifier used only as a
   `when=` condition. Both are arguable; neither is needed for the approved
   behavior, so they are listed rather than assumed.
2. **Does the kebab-specific quick-fix survive Requirement 1?** Once
   `{{{spec-name}}}` resolves, the diagnostic only fires for spellings the new rule
   still splits. Keep it, or drop it as dead weight?

## Success Criteria

1. A document with a kebab-case frontmatter key resolves `{{{ key-name }}}`,
   `{{{ doc.key-name }}}`, and `{{{ doc['key-name'] }}}` to the same value.
2. `{{{ _loop_count - 1 }}}`, `{{{ 4-2 }}}`, `{{{ f(x)-1 }}}`, and `{{{ arr[0]-1 }}}`
   all still evaluate as arithmetic.
3. A body expression that cannot be parsed or evaluated exits non-zero, names
   the source line, and never emits the `{{{ … }}}` into the output.
4. Every problem is reported exactly once: one bad expression is one message,
   and one undeclared identifier is one warning however often it is referenced.
5. An unresolved identifier that `$schema` does not declare **warns** — it is
   not an error — composition succeeds, and it still renders as an empty string.
   One that `$schema` does declare is silent.
6. DMLS flags an unknown identifier in an operand position, in both a body
   interpolation and a frontmatter expression value, at `WARNING` severity.
7. Hover and completion work anywhere inside a kebab identifier.
8. `prompts/_reviews/feature-review.md` would have failed to compose at the
   moment the original typo was introduced.

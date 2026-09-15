---
created: 2026-09-15
status: draft
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-15
---

# Dasherized Identifiers

Darkmatter's expression grammar cannot reference a kebab-case frontmatter key.
`{{ spec-name }}` parses as `spec` minus `name`, because `-` is only ever the
subtraction operator and identifiers are `[alphabetic|_][alphanumeric|_]*`
([`lexer.rs:952-960`](../../lib/src/markdown/compose/expression/lexer.rs)).

Kebab keys are ordinary YAML and ordinary frontmatter. The grammar must support
them.

This spec also fixes the reason the problem went unnoticed for so long: a body
interpolation that fails to parse or evaluate currently warns and passes the
`{{ … }}` through to the output instead of failing composition. Triple-brace
`{{{ … }}}` spans are interpolation literals and remain inert; they are not an
expression surface and are unaffected by this feature.

The current parsing design — the three stages, the identifier rules, the
grammar ladder, and the failure contract — is documented under
[`docs/topics/parsing/`](../../docs/topics/parsing/index.md). That documentation
is the "before" picture this spec changes.

## Status

Reviewed draft. The recommended answer to the remaining open question is
implementation-ready unless maintainers choose a broader suppression policy.

## Motivation

### The incident

`prompts/_reviews/feature-review.md` declared `spec_name` in its schema and
referenced it in the body as `{{spec-name}}`. Composition emitted:

```text
warning: [interpolation] failed to evaluate 'spec - name': Subtraction requires numeric operands
```

and shipped the literal text ``the `{{spec-name}}` spec`` into the prompt that
was handed to a review agent. Three separate failures lined up:

1. The grammar could not express what the author meant.
2. The runtime downgraded the failure to a warning and emitted the broken text.
3. The language server said nothing while the file was being edited.

All three are in scope here.

### Kebab keys are common, and not always ours to rename

The motivating shipped prompt is not isolated: historical specs and Claudine
documentation record the same `{{review-file}}` and `{{absolute-filepath}}`
mistake. Spaced arithmetic such as `{{_loop_count - 1}}` also exists. A raw
brace regex is not valid compatibility evidence, however: it includes inert
triple-brace literals, fenced documentation examples, and GitHub Actions
`${{ fromJSON(inputs.l2-environments) }}`, whose grammar is not Darkmatter's.

Before implementation, add or extend a parser-backed shipped-artifact audit
that classifies only executable Darkmatter surfaces: body and frontmatter
interpolations, `when=` expressions, Expression-typed frontmatter values, and
`$()` ternary conditions/branches. Record every currently valid unspaced
subtraction of the breaking identifier-like form. The implementation may rely
on the current finding that no shipped executable expression uses that form only
after this audit proves it; string literals such as `'review-' + iteration` do
not count because their dash is never tokenized as an operator.

Kebab frontmatter keys in the repo include `depends-on`, `supersedes-revision`,
`document-schema`, and — in `.claude/commands/` — `argument-hint` and
`allowed-tools`, which belong to Claude Code's frontmatter schema and cannot be
renamed to suit Darkmatter.

### An escape hatch exists but nothing points at it

`{{ doc['spec-name'] }}` resolves correctly today (verified). It is undiscoverable:
neither the warning text, the docs prior to this change, nor DMLS mentions it,
and the two spellings an author reaches for first — `{{spec-name}}` and
`{{doc.spec-name}}` — both fail the same silent way.

## Goals & Non-Goals

**Goals**

- A kebab-case frontmatter key is referenceable by its own name in every
  expression surface.
- A malformed or unevaluatable expression fails full-document composition on
  every executable expression surface.
- An identifier that names nothing is diagnosed by DMLS wherever it appears in
  an expression, not only at the root.
- A reference to an unknown bare root is surfaced as a **warning**, not an
  error, and never silently unless the author explicitly handles its absence.
- Each problem is reported exactly once per source document and compose run.

**Non-Goals**

- Whitespace-significant operators beyond the single rule below.
- Changing how unresolved-but-well-formed variables **render**. They stay an
  empty string and composition still succeeds; Requirement 4 adds a warning
  beside that output, not a different output.
- Renaming or normalizing existing kebab keys. They are valid input.
- Any change to operator/function result semantics, the function catalog, or
  namespace membership. Failure disposition and diagnostics intentionally
  change as specified below.
- Removing the explicit lenient contract from `compose_subtree`; strict subtree
  composition remains available to callers that require failure on malformed
  or unknown inputs.

## Requirement 1 — `-` may continue an identifier

### The rule

> A `-` continues the current identifier if and only if the lexer is already
> **mid-identifier** and the next character continues an identifier
> (`char::is_alphanumeric()` or `_`). Otherwise `-` is an operator.

Stated for authors: **binary minus requires whitespace before it**, unless the
left operand ends in `)`, `]`, a string delimiter, or a digit belonging to a
number literal. A digit at the end of an identifier does not create that
exception: `phase-2` is one identifier after this change.

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
| `"x"-1`, `false - 1` | subtraction | subtraction |
| `false-1`, `true-value` | subtraction / parse error | **identifier** (breaking) |
| `-5`, `a * -1` | unary minus | unary minus |
| `iteration-1` | subtraction | **identifier** (breaking) |
| `foo--bar` | `foo - (-bar)` | `foo - (-bar)` (second `-` cannot continue) |

An identifier-like left operand followed immediately by `-` and another
identifier-continuation character is the breaking class; `iteration-1` is its
most likely form. Boolean literals are reclassified only after `read_variable`
finishes, so `false-1` and `true-value` join too. The corpus contains no
known instance of this unspaced subtraction class; Phase 0 must prove that with
the parser-backed audit before release. Arithmetic with an identifier or boolean
left operand must use whitespace before the minus, for example
`{{ iteration - 1 }}` or `{{ false - 1 }}`.

### Both loops, not just the first

`read_variable` scans a leading identifier and then dotted path segments in a
second loop. Both must accept the joined `-`, or `doc.spec-name` stays broken
while `spec-name` works.

### The bracket form remains

`doc['spec-name']` keeps working and stays the way to reach a key the identifier
grammar cannot spell — a key containing `.`, `--`, a leading digit, or a
trailing `-`.

The `true` and `false` spellings remain boolean literals only when the complete
token is exactly that word. Supporting keys such as `true-value` is intentional;
it does not change the meaning of standalone boolean literals.

## Requirement 2 — Invalid expressions fail document composition

An executable expression that cannot be parsed or evaluated is an authoring
error on every full-document composition surface. Body interpolation and mixed
text in frontmatter must stop degrading those failures to warnings.

| Surface | Parse error | Evaluation error | Today |
| --- | --- | --- | --- |
| Frontmatter whole-value interpolation | fatal, exit 1 | fatal, exit 1 | correct |
| Frontmatter mixed-text interpolation | fatal, exit 1 | fatal, exit 1 | **lenient when `fail_fast` is false** |
| `when="…"` conditions | fatal, exit 1 | fatal, exit 1 | correct |
| `$()` ternary conditions and branches | fatal, exit 1 | fatal, exit 1 | correct |
| Body interpolation | fatal, exit 1 | fatal, exit 1 | **warns, emits `{{ … }}` verbatim, exit 0 when `fail_fast` is false** |

The public `ComposeOptions::with_fail_fast(false)` option remains meaningful for
recoverable non-expression stages such as TOC linking and non-structural
transclusion. It no longer authorizes malformed or unevaluatable expressions in
a full document. This is an intentional narrowing of that option and requires
updates to its API documentation, `ComposeWarning` documentation, the fatality
characterization matrix, and the Darkmatter skill's error-handling section.

Do not implement the change by making the shared `interpolate_text` helper
unconditionally strict. `compose_subtree(..., SubtreeStrictness::Lenient)` is an
established public contract used for best-effort data-tree interpolation. The
document pipeline must pass an explicit expression-failure policy (or use a
document-specific wrapper), while `SubtreeStrictness::{Lenient, Strict}` retains
its current behavior. This also prevents the change from silently altering
Claudine's explicit subtree choices.

The error must carry the original source path, line, and expression span and
render through the same typed rich-error surface as other compose errors. No
partially rewritten document or verbatim failing span may be emitted on stdout.
The body stage currently constructs an effective-source error without attaching
the document's source context; the implementation must project the scanner's
byte span through that context rather than merely adding a line number to the
message.

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

`{{spec-name}}` parses to `Binary { Variable("spec"), Sub, Variable("name") }`,
so `root_identifier` returns `None` and the check silently no-ops. Every
identifier in an operand position is unchecked: `{{a - b}}`, `{{x ? y : z}}`,
`{{f(arg)}}`, `{{a || b}}`.

The fix is to walk the AST and diagnose every unknown `Variable` node at its own
span, subject to Requirement 4's explicit-absence suppressions. The evaluator
already contains the traversal shape for the `ctx.*`
namespace — `walk_context_variables` / `collect_context_warnings` in
[`interpolation/evaluator.rs`](../../lib/src/markdown/compose/interpolation/evaluator.rs)
— which recurses Binary, Ternary, Fallback, Index, and MemberAccess and reports
`unknown context variable 'ctx.foo'` with a did-you-mean. The new walk should
share that shape.

This matters more after Requirement 1, not less: `{{iteration-1}}` becomes a
well-formed reference to a key that does not exist. Requirement 4 catches it at
compose time; this requirement is what catches it while the author is still
typing, and what catches the same class of typo in an operand position
regardless of dashes.

The diagnostic walk must be a new helper. It must not replace or change
`root_identifier`: GitNexus reports five direct consumers, including hover,
go-to-definition, graph indexing, and both diagnostic providers. Those other
consumers intentionally ask for one navigable root and would change semantics
if they suddenly received every operand. Graph indexing can gain multi-variable
facts in a separate feature, but that is not required to diagnose expressions.

For each `Variable` node, classification uses its first dotted segment. The
reserved `ctx`, `env`, and `doc` roots remain known namespaces; existing
namespace-specific diagnostics continue to own invalid members so this feature
does not emit a duplicate unknown-identifier diagnostic. A frontmatter root is
known when it is present in the document or declared by DMLS's effective schema
shape. Checking deeper object members requires schema-aware path analysis and is
outside this feature.

### Dash-separated-key diagnostic

When the authored source for a subtraction can be normalized to a frontmatter
key that actually exists or is declared — for example `a- b` matching `a-b`, or
`foo--bar` matching `foo--bar` — DMLS may identify a mis-referenced key rather
than generic unknown operands. DMLS has the required membership information
(`is_unknown_identifier` at [`dsl.rs:891`](../../dmls/src/providers/dsl.rs) uses
`ast.entry_by_dotted`). It should say so only on an exact source/key match and
offer one semantics-preserving quick-fix: remove the separating whitespace when
the resulting key is legal under Requirement 1, otherwise use bracket access
such as `doc['foo--bar']`. Do not claim support for documents "pinned" to the old
grammar; no expression grammar-version mechanism exists.

Keep the stable `dm.expression.unknown_identifier` code and attach structured
diagnostic data describing the exact replacement; the code-action provider must
not reparse the human-readable message. If a safe whole-expression replacement
cannot be proven, emit the ordinary diagnostic without a quick-fix. When the
exact key match is proven, emit one key-level diagnostic for the subtraction
span instead of separate generic diagnostics for its operand variables.

## Requirement 4 — An unresolved unknown root warns

A well-formed identifier that resolves to nothing is **not** an error. Its type
is `any` by default, and `null`/undefined is a valid inhabitant of `any`, so
composition proceeds and it renders as an empty string exactly as it does today.
But in most documents it is a typo, so it must not be silent either.

The deciding question is whether the root name is **known** to the request. A
root is known when it is present in effective state (including an explicit
`null` or empty-string value, `--set`, inherited state, or a caller input layer),
is a reserved namespace, or is declared by the effective schema. The effective
schema includes the document's `$schema`, configured baseline, and matched
schema extensions/triggers; the document-local `$schema` is not the only schema
authority in Darkmatter.

The distinction between missing and falsy is mandatory. `EvaluationLookup::get`
already distinguishes `None` from `Some(Value::Null)` and
`Some(Value::String(""))`; the warning must use presence/known-root information,
not the rendered empty string. If additional lookup metadata is needed, extend
the lookup contract explicitly rather than reconstructing schema state inside
the evaluator.

This rule applies wherever full-document composition evaluates variables:
frontmatter interpolation in both passes, body interpolation, `when=`
conditions (including transclusion/page-block conditions), and `$()` ternary
conditions or branches. Use one shared classifier/collector so these surfaces
cannot drift. Passive schema parsing and DMLS validation perform no evaluation;
DMLS applies the corresponding static rule from Requirement 3.

### Preserve compose ordering with deferred candidates

Frontmatter interpolation pass 1 runs before schema validation and before the
final effective state exists. It therefore cannot decide whether a missing root
is declared by the eventual effective schema or supplied during shell/pass-2
processing. Do not reorder composition or resolve schema inside the evaluator.
Instead, evaluation records a source-spanned unknown-root **candidate** in a
request-scoped diagnostic accumulator. Reconcile candidates after the final
frontmatter interpolation pass and again when child/transclusion reports merge:
discard any root then known to effective state/schema and emit the remaining
deduplicated warnings. Later body/condition evaluations use the same accumulator
and may classify immediately because their effective state is available.

Candidate collection is observational only: it must not execute an expression a
surface would not otherwise evaluate, perform schema or file I/O, or recapture
request context. The accumulator travels with the existing compose request and
source provenance.

### Worked examples

In every row, `iteration-1` has no value in frontmatter and none supplied by the
caller.

| Expression | `$schema` | Result | Why |
| --- | --- | --- | --- |
| `{{ iteration-1 }}` | not declared | **warning**, renders empty, exit 0 | nothing vouches for the name |
| `{{ iteration-1 \|\| "fallback" }}` | not declared | silent, renders `fallback` | the author handled the absence |
| `{{ iteration-1 }}` | `iteration-1: number` | silent, renders empty | declared and optional, so `null` is in the type |
| `{{ iteration-1 }}` | `iteration-1: number(required)` | **fatal**, exit 1 | not this requirement — see below |

### A required property is already handled

A required property left unset fails schema validation before body interpolation
and before unknown-root candidates are reconciled:

```text
missing iteration-1: required but not provided
```

Requirement 4 must **not** add a warning for that case. The property is known,
so the schema gate already owns it, and a second message about the
same missing value would violate
[Requirement 5](#requirement-5--one-diagnostic-per-issue). The known-root gate in
Requirement 4 therefore keys on *known or unknown*, and never on required-ness;
schema validation owns the required-property failure.

### Current behavior, for contrast

Verified today: a declared-optional-unset reference and an undeclared-unset
reference both render empty and exit 0 with no diagnostic whatsoever. Requirement
4 changes exactly one of those two — the undeclared one.

This is the missing runtime half of a rule DMLS already implements. Its
`is_unknown_identifier` ([`dsl.rs:880-903`](../../dmls/src/providers/dsl.rs))
already treats a schema-declared property as known "even when the document
leaves it unset", and already exempts `ctx.*` / `env.*` / `doc.*` roots and
function names. Requirement 4 gives the runtime the same root-classification
semantics, while allowing runtime-only input layers that DMLS cannot observe.

### Where the runtime is better informed than the editor

DMLS declines to fire at all on a frontmatter-less document, because any bare
identifier there could be a `--set` value it cannot see. Preserve that editor
exception. The runtime has no such blind spot: it classifies the effective
request after all caller-provided layers have been applied, so a root supplied
by `--set`, inherited parent state, or another caller input never warns even
when its value is explicitly null or empty.

### Explicitly-handled absence must stay silent

`{{ color || "unknown" }}` and `{{ color ? color : "none" }}` are the
documented idioms for a value that may be absent
([`inline/interpolation.md`](../../docs/inline/interpolation.md)). Warning on
them would punish the author for handling the case correctly and would make the
warning worthless through noise.

The warning is suppressed when the unresolved identifier is the **primary of a
`Fallback`** node or the **condition of a `Ternary`**. Both are explicit
"this may be absent" constructs. When the ternary condition is a bare variable,
matching references to that same root in its branches are guarded by the
condition and are suppressed too; this keeps the documented
`color ? color : "none"` idiom silent. A different unknown root in either branch
still warns.

| Expression | Unresolved `x` warns? |
| --- | --- |
| `{{ x }}` | yes |
| `{{ x \|\| "d" }}` | no — primary of a fallback |
| `{{ a \|\| x }}` | yes when the right-hand side is evaluated |
| `{{ x ? a : b }}` | no — ternary condition |
| `{{ x ? x : b }}` | no — the branch reference is guarded by `x` |
| `{{ a ? x : b }}` | yes when the `x` branch is evaluated |

Runtime warnings follow evaluation reachability: an unchosen ternary branch or
short-circuited fallback is not a runtime issue and must not warn. DMLS remains
static and may diagnose an unknown identifier in either non-suppressed branch,
but it applies the same fallback-primary, ternary-condition, and guarded-root
suppressions so editor and runtime do not disagree about intentional absence.
The runtime collector must preserve the evaluator's short-circuit behavior
rather than blindly walking the whole AST before evaluation.

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

One expression that fails is one message, no matter how many times the rewrite
engine scans it. The current rescan loop explains the duplicate: a failing span
is left unchanged; if another span is successfully replaced, the next depth
rescans the unchanged failure and emits it again. This is not the proxy/redirect
path and does not depend on `$schema`.

Requirement 2 removes this particular warning from full-document composition,
but the rescan contract still matters for unresolved-root and `ctx.*` warnings.
Track diagnostic identity independently of mutable output offsets. A diagnostic
key is `(source document identity, diagnostic code, normalized root name)` for
root warnings and `(source document identity, diagnostic code, original source
span)` for expression failures. Keep the first authored location for display.
Replacement-generated expressions have the source identity of the document and
the first generated span at which they are observed; they are not allowed to
erase or alias an authored diagnostic.

### Per identifier, not per occurrence

An unknown, unresolved root referenced ten times in one source document is one
mistake and produces **one** warning, naming the root and the location of its
first authored occurrence. The same root in two transcluded source documents is
two independently actionable issues and produces one warning per source. Two
different unknown roots produce two warnings.

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
| 2 | [`dmls/src/overlay/expressions.rs`](../../dmls/src/overlay/expressions.rs) cursor/partial helpers | use grammar-consistent identifier spans so hover/completion/definition include joined dashes without treating `--`, a trailing dash, or subtraction as one identifier |
| 3 | document interpolation call sites and typed error projection | make body and mixed-frontmatter expression failures fatal without changing lenient subtree composition; attach source path and span (Requirement 2) |
| 4 | [`overlay/expressions.rs`](../../dmls/src/overlay/expressions.rs) + [`dsl.rs`](../../dmls/src/providers/dsl.rs) + [`diagnostics/frontmatter.rs`](../../dmls/src/diagnostics/frontmatter.rs) + [`code_actions.rs`](../../dmls/src/providers/code_actions.rs) | add a full-AST variable walk for the two diagnostic call sites while preserving `root_identifier` for navigation/graph indexing; carry a structured safe replacement into the quick-fix provider (Requirement 3) |
| 5 | request-scoped diagnostics, effective-state/schema handoff, evaluator, and condition/`$()` call sites | collect source-spanned candidates, reconcile them against final request state/schema, and emit evaluation-aware unresolved-root warnings through one shared policy (Requirement 4) |
| 6 | [`dsl.rs:696`](../../dmls/src/providers/dsl.rs), [`diagnostics/frontmatter.rs:653`](../../dmls/src/diagnostics/frontmatter.rs) | raise `dm.expression.unknown_identifier` from `INFORMATION` to `WARNING` |
| 7 | interpolation rescan and `ComposeReport` merge paths | preserve source provenance and deduplicate by the stable identities in Requirement 5 |

DMLS parses through darkmatter's own `parse_spanned` / `parse_condition_spanned`,
so the AST, spans, and both expression diagnostics follow the lexer change.
Its ad hoc completion and cursor word scans do not follow automatically: they
must reuse token/AST spans or implement the exact joined-dash rule. Adding `-`
to a global word character class is incorrect because it would merge
`foo--bar` and subtraction. `--set` key validation and schema assignment already
accept `-`.

**Side effect, intentional.** Unquoted object-literal keys gain kebab support:
`{ spec-name: 1 }` becomes legal, because the key guard at
[`parser.rs:592`](../../lib/src/markdown/compose/expression/parser.rs) accepts a
single `Variable` token with no dot and an ASCII alphabetic-or-`_` first
character. This is consistent with the rest of the change. Add parser coverage,
including that a quoted key remains required where the existing ASCII-first
guard requires one.

## Compatibility

- **Grammar.** `iteration-1` changes meaning. The parser-backed compatibility
  audit must confirm that no shipped executable expression relies on that form
  before release. Its new meaning is an unknown, unresolved identifier, so
  Requirement 4 warns at compose time and Requirement 3 flags it in the editor.
  Requirement 1 must not ship without those diagnostics.
- **Runtime.** Requirement 2 turns previously-passing composes into failures
  wherever a body or mixed-frontmatter expression was already broken, even with
  `ComposeOptions::with_fail_fast(false)`. That is the point, but it will surface
  latent breakage on first run, including `{{absolute-filepath}}` and
  `{{review-file}}` — both of which Requirement 1 simultaneously fixes. Explicit
  lenient subtree composition does not change.
- **Diagnostics.** DMLS moves unknown-root diagnostics from information to
  warning and finds roots in operand positions. Clients that filter by severity
  will observe more warnings; the stable diagnostic code does not change.
- **Public API documentation.** No signature must change solely for this
  feature, but any new failure-policy or known-root input added to shared
  internals must not default third-party `EvaluationLookup` implementations into
  false warnings. Preserve the trait's compatibility defaults.
- **Persisted output.** None. No AST variant, serialized form, or public
  signature changes.

## Implementation Plan

### Phase 0 — Compatibility inventory

Extend the passive shipped-artifact corpus to classify every executable
Darkmatter expression surface and pin the current unspaced-subtraction
inventory. Resolve any real collision before changing the lexer; do not count
GitHub Actions syntax, interpolation literals, fenced examples, or string
contents as Darkmatter operator use.

### Phase 1 — Lexer

The `read_variable` rule, plus grammar-consistent DMLS cursor/partial handling in
the same change so hover, completion, and definition do not regress.

### Phase 2 — One diagnostic per issue

Requirement 5's source-aware diagnostic identity and rescan deduplication.
Sequenced before the body-failure change so the current duplicate remains a
useful regression fixture rather than being masked by the switch to a fatal
error.

### Phase 3 — Fatal body failures

Requirement 2 for full-document body and mixed-frontmatter interpolation,
including typed source-span projection and the preserved subtree strictness
contract. Independent of Phase 1; sequence either way.

### Phase 4 — Unresolved-identifier warning

Requirement 4: introduce the request-scoped candidate accumulator; reconcile
pass-1 candidates only after final state/schema is known; route interpolation,
condition, and `$()` evaluation through the shared policy; preserve
short-circuit reachability; apply the approved structural suppressions; align
DMLS severity. Benefits most from landing after Phase 1, so valid kebab
references already resolve.

### Phase 5 — DMLS identifier walk

Requirement 3's diagnostic-only full-AST walk, shared by the body and
frontmatter call sites, plus the exact dash-separated-key message and quick-fix.

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
  known to effective state/schema or the absence is explicitly handled
- public `ComposeOptions::with_fail_fast`, `ComposeWarning`, and subtree
  strictness docs — distinguish expression authoring failures from other
  recoverable compose failures and preserve explicit lenient subtree behavior
- `.claude/skills/darkmatter/compose.md` — the parsing pointer and error policy

## Testing Requirements

**Lexer (L1).** Every row of the Requirement 1 table as a token-stream
assertion, plus: `-` at end of input; `-` followed by whitespace; `-` followed by
`-`; a kebab segment inside a dotted path; a kebab identifier as an unquoted
object key; `café-name` (the identifier predicates are Unicode).

**Parser (L1).** The raw expression `iteration - 1` still lowers to `Binary`; `4-2`,
`f(x)-1`, `arr[0]-1` still lower to `Binary`; `spec-name` lowers to a single
`Variable` with a span covering the whole name.

**Compose (L1).** A document with a kebab frontmatter key renders it bare, via
`doc.<key>`, and via `doc['<key>']`. Regression: `{{ _loop_count - 1 }}` still
evaluates.

**Failure contract (L1).** At the library boundary, cover body and mixed-text
frontmatter parse/evaluation failures with `fail_fast` both false and true, plus
the already-strict whole-value, `when=`, and `$()` surfaces. Assert a typed error
with source path/span and no partially rewritten output. Separately, use
`CliProcessFixture` for representative `md compose` body and mixed-frontmatter
failures; assert exit 1, a source line, no verbatim failing `{{ … }}` on stdout,
and exactly one rendered error. Add a subtree regression proving Lenient stays
lenient and Strict stays strict.

**Diagnostic count (L1).** Preserve the original two-span rescan fixture: one
successful replacement plus one bad expression produces exactly one issue. One
unknown root referenced ten times in a document produces exactly one warning.
The same root in two transcluded source documents produces two, and two
different roots produce two.

**Unresolved identifiers (L1).** One case per row of the
[worked-examples table](#worked-examples), asserting exit code, rendered output,
and diagnostic count together — including that the `number(required)` row fails
with the schema's own message and **no** additional unresolved-identifier
warning. Every row of the
[suppression table](#explicitly-handled-absence-must-stay-silent), asserting the
evaluated branch/right-hand-side positions warn and unreachable ones do not.
Assert no warning when a root arrives via `--set`, inherited state, another
caller layer, baseline/trigger schema, or as an explicit null/empty value.
Cover a body expression, mixed frontmatter value, page-block `when=`,
transclusion `when=`, and `$()` ternary with representative unknown roots so
every runtime call site is wired to the shared policy.

**DMLS (L1 protocol/provider tests).**
`dm.expression.unknown_identifier` fires for an unknown identifier in each
operand position: binary operand, ternary branch, function argument, fallback
right-hand side — in both a body interpolation and a frontmatter expression
value. Hover, completion, and go-to-definition resolve on a kebab identifier at
every cursor position within it, including on the dash; `foo--bar`, a trailing
dash, and spaced subtraction are not merged by cursor scans. Assert the
explicit-absence suppressions and the frontmatter-less editor exception. For the
dash-separated-key diagnostic, assert both replacement forms, the structured
diagnostic data, and that ambiguous arithmetic offers no fix.

These tests are L1 because none requires a real terminal, browser, device, or
host input. Run them with `just test` in `darkmatter/`; do not place them in
`just test-l2`. Because this changes a parser and a shipped prompt path, extend
the shared passive shipped-artifact corpus and keep one end-to-end composition
of `prompts/_reviews/feature-review.md` through its normal invocation path.

## Resolved Decisions

1. **`-` is not made an identifier character globally.** The join is decided
   inside identifier scanning. A global character class would make `4-2`
   ambiguous.
2. **`-` followed by a digit joins.** `phase-2` is a plausible YAML key; the
   current inventory has no known unspaced subtraction to protect (Phase 0 pins
   this before implementation), and the alternative leaves a second class of
   unreachable keys. The cost is `iteration-1`, which Requirement 3 diagnoses.
3. **The new ambiguity is resolved by whitespace before binary minus.** This is
   an intentional whitespace-sensitive disambiguation: `a-b` becomes one name,
   while `a -b` and `a- b` remain subtraction. Already-spaced expressions do not
   change meaning.
4. **Invalid executable expressions become fatal in full-document compose.**
   This is an intentional behavior change, not comment drift being resolved in
   favor of prose. `with_fail_fast(false)` remains available for other
   recoverable stages, and explicit lenient subtree composition is preserved.
5. **An unresolved unknown root warns rather than failing.** Unknown means
   absent from effective state, reserved namespaces, and the effective schema —
   not merely absent from document `$schema`. Failing would be wrong on the
   `any`/null type rules alone, but silence is wrong because most such references
   are typos. This keeps Requirement 2 (evaluation errors are fatal) and
   Requirement 4 (unknown absence is a warning) separate.
6. **Explicit absence handling suppresses the warning.** A fallback primary and
   ternary condition are suppressed, as are matching guarded references in the
   ternary branches. Other evaluated positions still warn. DMLS applies the
   same structural suppressions; runtime additionally honors short-circuit
   reachability. Approved 2026-09-15.
7. **Keep a narrow dash-separated-key quick-fix.** Requirement 1 removes the
   common `spec-name` case but not keys such as `foo--bar` or an accidentally
   spaced `a- b`. An exact match to a present/declared key is strong enough to
   provide a specific message; all other arithmetic retains generic diagnostics.
8. **Diagnostic walking does not replace `root_identifier`.** Navigation and
   graph-index consumers retain their one-root contract. Only the two diagnostic
   providers use the new all-variable walk.

## Open Questions

1. **Should recognized absence predicates suppress unknown-root warnings?**
   `is_null(x)` and `is_empty(x)` can deliberately inspect absence, while a bare
   unknown `when="x"` is just as likely to be a typo as any other reference.

   - **Suppress predicate arguments and bare `when=` roots.** Pros: maximally
     permissive for conditional authoring. Cons: a misspelled gate silently
     disables content, recreating the motivating failure class.
   - **Suppress only direct arguments to recognized absence predicates
     (recommended).** Pros: models explicit intent without hiding misspelled
     `when=` conditions; it is a small, catalog-driven rule. Cons: adding a new
     absence predicate requires updating the suppression catalog.
   - **Suppress neither.** Pros: smallest implementation and strongest typo
     detection. Cons: warns on the clearest function-based way to ask whether a
     value is absent.

   **Recommendation:** suppress only a direct unknown argument to the canonical
   `is_null` / `is_empty` functions or their registered aliases, and keep bare
   `when=` roots warning. These functions state the author's absence intent
   explicitly; a bare condition does not. Resolve aliases through the function
   catalog rather than duplicating strings in the diagnostic walker. Nested
   uses such as `is_empty(trim(x))` still warn for `x` because the predicate is
   no longer directly guarding that lookup.

## Success Criteria

1. A document with a kebab-case frontmatter key resolves `{{ key-name }}`,
   `{{ doc.key-name }}`, and `{{ doc['key-name'] }}` to the same value.
2. `{{ _loop_count - 1 }}`, `{{ 4-2 }}`, `{{ f(x)-1 }}`, and `{{ arr[0]-1 }}`
   all still evaluate as arithmetic.
3. A body or mixed-frontmatter expression that cannot be parsed or evaluated
   fails full-document composition regardless of `fail_fast`, carries its source
   path/span, and never emits partial output; lenient subtree behavior is
   unchanged.
4. Every problem is reported exactly once: one bad expression is one message,
   and one unknown root is one warning per source document however often it is
   referenced there.
5. An unresolved root absent from effective state, reserved namespaces, and the
   effective schema **warns** — it is not an error — composition succeeds, and
   it still renders as an empty string. A known root is silent even when its
   value is null or empty.
6. When document context permits static classification, DMLS flags an unknown
   identifier in an operand position, in both a body interpolation and a
   frontmatter expression value, at `WARNING` severity; the documented
   frontmatter-less exception remains silent.
7. Hover, completion, and go-to-definition work anywhere inside a kebab
   identifier without merging subtraction or unsupported dash forms.
8. `prompts/_reviews/feature-review.md` would have failed to compose at the
   moment the original typo was introduced.

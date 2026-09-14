---
created: 2026-09-13
status: draft
reviewed: false
implemented: false
area: claudine
packages:
    - darkmatter
    - dmls
    - claudine
    - claudine-cli
related:
    - claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/spec.md
    - claudine/docs/topics/lifecycle.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/lsp/features.md
---

# Reject Nested `{{ … }}` Inside Expression String Literals Before Anything Runs

## Outcome

An author who writes a `{{ … }}` span inside a quoted string literal of a
Darkmatter expression, on a surface that is evaluated once, finds out in
three places, in this order, and never after an agent has already run:

1. **In the editor.** DMLS underlines the inner braces with a warning that
   says the braces are literal text, and offers a quick fix that rewrites
   the literal as `+` concatenation.
2. **At prepare time.** Claudine refuses to compose the document. The error
   names the lifecycle key (`success.say`), quotes the literal, highlights
   the frontmatter line, and prints the `+` rewrite. No provider is spawned.
3. **At event time**, only for the cases static analysis cannot see (a
   frontmatter value that holds raw template text at runtime). That message
   also names the lifecycle key and says in plain words what survived and
   why.

One rule, owned by Darkmatter, produces all three verdicts. The static
verdict and the event-time guard recognize a nested span with the same
scanner, and they classify a surface as rescanning or non-rescanning with
the same predicate, so they can never disagree about what counts as a
defect.

## Observed Incident

On 2026-09-13 `prompts/review.md` proxied to
`prompts/_reviews/review-spec-inline.md` for an inline review of
`fixes/2026-09-13-cicd-redundancies/spec.md`. The review ran to completion
under Codex and marked the spec `reviewed: true`. Then the `success`
lifecycle event fired and the run exited non-zero with:

```text
CompositionError: lifecycle evaluation error

A late-binding expression raised while the `success` lifecycle event was
firing, in an interpolated string (prompts/_reviews/review-spec-inline.md).

Reason: unresolved interpolation survived event-time resolution: `The review
of the draft specification file in the {{ctx.repo_name}} repo has completed`

This is a crashed expression, not a clean `false` guard: the run halts and
exits non-zero. Fix the expression (resolve the missing path or variable,
correct the function call) or guard it with a fallback so it evaluates
instead of raising.
```

The authored `success.say` value was:

```yaml
success:
    say: |-
        {{
        ctx.area
            ? "The review of the draft specification file in {{ctx.area}} has completed"
            : "The review of the draft specification file in the {{ctx.repo_name}} repo has completed"
        }}
```

The `failure.say` value in the same file had the same shape. Had the review
failed, this crash would have replaced the real failure report.

`prompts/commit.md` carries a second live instance of the same defect, in a
different and more demanding shape:

```yaml
resides_in: |-
    {{
        length(ctx.dirty_package_areas) == 1
            ? 'are all part of the {{ctx.dirty_package_areas}} package area'
            : 'are spread across {{length(ctx.dirty_package_areas)}}:\n {{as_unordered_list(ctx.dirty_package_areas)}}'
    }}
```

Single quotes rather than double, an escaped `\n` in the same literal as two
nested spans, and a span (`ctx.dirty_package_areas`) whose value is an
array. Every awkward case in D1 appears in this one value, which is why it
becomes the second regression fixture.

## Root Cause

Three facts combine, and none of them is documented today:

- **The span finder is brace-depth aware.** `ExpressionFinder::scan_legacy_expression`
  (`darkmatter/lib/src/markdown/compose/expression/lexer.rs`) counts nested
  `{{` / `}}` pairs, so the whole ternary above is one expression rather
  than being cut at the first `}}`.
- **String literals are inert, but the interpolator rescans its own
  output.** The lexer's `read_string` copies every character between quotes
  verbatim, so `{{ctx.repo_name}}` inside a quoted string is text at parse
  time. That is not the whole story. `interpolate_text`
  (`darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`) is a
  fixpoint loop over `MAX_INTERPOLATION_DEPTH` that rescans the text it just
  produced, so a literal's value lands in the output and the next depth pass
  interpolates the braces it carries. On a document body, and in a mixed
  frontmatter string, this construction therefore resolves correctly. The
  bypass is `whole_value_span`: when a scalar's trimmed content is exactly
  one span, `interpolate_value` calls `parse` and `eval_json` directly,
  once, with no rescan. That is the path every lifecycle value takes
  (`resolve_string_value` → `SubtreeCompose` → `compose_string` →
  `interpolate_value`), and it is what makes the defect fatal rather than
  invisible.
- **The runtime leak guard catches the symptom, not the cause.**
  `reject_surviving_spans` (`claudine/lib/src/composition/lifecycle/executor.rs`)
  was written for a different scenario: a frontmatter value that is itself
  raw template text (for example, a `set_frontmatter` that stored `"{{x}}"`).
  Its message describes what it was built to catch. Its hint ("resolve the
  missing path or variable") points the author away from the bug, because
  nothing was missing.

The ternary evaluated, `ctx.area` was the empty string for a repo-root spec,
the else branch returned a literal containing `{{ctx.repo_name}}`, the
whole-value path did not rescan it, and the guard raised. Every one of those
steps worked as designed. The defect is that a construction which can never
evaluate correctly on a non-rescanning surface was accepted at parse time,
at prepare time, and in the editor.

### Why prepare-time validation missed it

`validate_no_undefined_lifecycle_variables` and
`validate_no_err_in_no_error_events`
(`claudine/lib/src/composition/lifecycle/validate.rs`) walk the parsed
`Expr` tree of every lifecycle surface. Neither looks at the *source text* of
a `StringLiteral` node for braces. The one helper that does,
`visit_string_literals`, is used to find surviving spans and `err`
references, and it deliberately treats braces inside literals as
interpolable, because Claudine synthesizes an `Expr::StringLiteral` for every
positional action body (`render_message`, executor.rs). Those synthesized
literals **are** interpolated at event time. An authored literal between
quotes inside `{{ … }}` is not. The tree cannot tell the two apart; the
source text can.

A fourth validator already looks like the missing one and is not wired up.
`validate_no_interpolation_leaks` (same file) walks
`iter_stack_expression_surfaces` with `visit_string_literals` hunting for
surviving spans in literals, is exported from `lifecycle/mod.rs`, and has
tests in `lifecycle/tests/diagnostics.rs` and `lifecycle/tests/validation.rs`.
No production call site exists for it in `composition/prepare/` or in the
CLI, yet `claudine/docs/topics/lifecycle.md` states that it "still runs" for
non-lifecycle surfaces. Green tests over an uncalled function read as
coverage and are why this surface looked guarded. See Open Questions.

### Why DMLS missed it

DMLS diagnoses `{{ … }}` spans in two places only: the document body
(`providers/dsl.rs::expression_diagnostics`, which starts scanning at
`body_base`) and schema-typed `expression` scalars in frontmatter
(`diagnostics/frontmatter.rs::expression_diagnostics`, fed by
`expression_values`, which excludes plain strings). A lifecycle value such as
`say: |- {{ … }}` is a plain frontmatter string. DMLS never parses it, so it
could not have found a nested span even if the rule existed.

## Terminology

- **Authored literal.** A single- or double-quoted string token that appears
  in the source text of an expression, between `{{` and `}}`. Produced by the
  lexer; carried with its byte span in `SpannedExprKind::StringLiteral`.
- **Synthesized literal.** An `Expr::StringLiteral` that Claudine builds from
  a YAML scalar (a positional action body, a key/value parameter). It has no
  authored quotes and no source span inside an expression. Its `{{ … }}`
  spans are interpolated by design.
- **Nested span.** A `{{ … }}` span, as recognized by
  `ExpressionFinder::find_all_plain`, that lies inside an authored literal.
- **Expression surface.** Any place Claudine or Darkmatter parses text as an
  expression. Surfaces fall into two tiers, and the whole of this fix turns
  on the division:
    - **Rescanning surfaces** are evaluated through `interpolate_text`, which
      loops to a fixpoint over its own output: a document body
      interpolation, and a mixed frontmatter string such as
      `"a {{ … }} b"`. A nested span on one of these is resolved by the next
      depth pass.
    - **Non-rescanning surfaces** are evaluated exactly once, with no rescan:
      a whole-value frontmatter scalar (trimmed content is exactly one span,
      routed to the `whole_value_span` branch of `interpolate_value`), every
      lifecycle communication field, every `when` / `until` / `while`
      predicate, every stack action operand, and every `proxy … with` value.

## Required Invariants

1. **A nested span is a defect on any surface that is evaluated once,
   without a rescan pass.** On a non-rescanning surface nothing ever
   re-interpolates an authored literal, the escape hatch for emitting
   literal braces is `{{{ … }}}`, and the runtime guard already rejects a
   surviving span, so there is no legitimate program the static rule would
   reject there. Rescanning surfaces resolve the same construction
   correctly and are deliberately outside the rule: that behavior is pinned
   by `rescans_replacement_text_for_nested_interpolation` and
   `rescans_false_branch_for_nested_interpolation` in `rewrite.rs`, and used
   as a live fixture at
   `darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/compose_interpolation_heavy.md:61`.
   Flagging a body interpolation would be a false positive against green
   tests.
2. **One recognizer, and one surface classifier.** The static rule and
   `reject_surviving_spans` decide "is this a span" with the same function.
   Darkmatter also exports exactly one predicate answering "is this surface
   evaluated once" — the whole-value-span shape `interpolate_value` already
   branches on — and Claudine, DMLS, and the lint all call it. Two things
   can desync here, not one, and both get the same treatment.
3. **Source text, not the tree.** The rule runs on `SpannedExpr` parsed from
   authored expression text. It never inspects a synthesized literal, so no
   positional action body is ever flagged.
4. **Fail before spawn.** Claudine rejects the document in the shared
   prepare stage (`prepare_document`,
   `claudine/lib/src/composition/prepare/service.rs`, which every route —
   direct, proxied, retried, resumed, loop-refreshed — arrives at), before
   the provider process exists. This includes the target of a `proxy`
   handoff, validated at the handoff before the target is launched, and
   every step of a sequence, validated by Phase 1c before the first step
   starts.

   Two exceptions are inherent to just-in-time composition and must not be
   read out of this invariant:
    - A sequence step whose document reference is itself interpolated
      (`task: "{{ … }}.md"`) cannot be resolved before the run starts. It is
      caught at that step's own turn, before that step's provider is
      spawned.
    - Phase 1c validates against the *initial* runtime state. An earlier
      step can rewrite a later step's document — inline-compose write-back,
      or an agent editing frontmatter mid-run. A defect introduced that way
      passes the preflight and is caught at the step's own turn. This is not
      fixable here.
5. **Editor and CLI agree.** The DMLS diagnostic range is the inner `{{`
   through its `}}`, and the Claudine error highlights the same frontmatter
   value and quotes the same literal. The suggestion is the **bare
   expression text** — no `{{ }}` wrapper, no YAML quoting — so the two
   consumers can assert byte equality on it; each is responsible for its own
   wrapping. Precise inner-brace ranges are promised for flow scalars and
   literal `|` block scalars; a folded `>` scalar is ranged on the whole
   scalar (D3).

## Design

### D1. Darkmatter owns the rule

Add an expression lint module under
`darkmatter/lib/src/markdown/compose/expression/` with these public entries:

```rust
pub struct ExpressionLint {
    pub kind: ExpressionLintKind,
    /// Byte span of the offending token inside the expression text.
    pub span: SourceSpan,
    /// A rewrite of the whole expression that removes the defect, when one
    /// can be produced mechanically. Bare expression text, no `{{ }}`.
    pub suggestion: Option<String>,
}

pub enum ExpressionLintKind {
    /// A `{{ … }}` span inside an authored string literal. Carries the
    /// literal's span and the nested span's text.
    NestedSpanInStringLiteral { literal: SourceSpan, nested: String },
}

pub fn lint_expression(source: &str, mode: ParseMode) -> Vec<ExpressionLint>;
pub fn lint_spanned(source: &str, expr: &SpannedExpr) -> Vec<ExpressionLint>;

/// True when the trimmed text is exactly one `{{ … }}` span: the shape
/// `interpolate_value` routes to `whole_value_span`, and therefore the
/// shape that is evaluated once. The sole authority for Invariant 2's
/// surface classification.
pub fn is_whole_value_span(text: &str) -> bool;
```

`lint_expression` parses with `parse_spanned` (or the condition-dialect
equivalent for `ParseMode::Condition`) and delegates to `lint_spanned`. A
parse failure yields no lints; malformed-expression diagnostics already
cover that path and must not be duplicated.

`lint_spanned` walks every `SpannedExprKind::StringLiteral` (object-literal
keys included, matching `visit_string_literals`) and runs
`ExpressionFinder::find_all_plain` over the literal's **source slice**
(quotes stripped, escapes left as authored). One lint per nested span. The
recognizer is the same function the runtime guard calls, which satisfies
Invariant 2.

Callers decide *whether* to run the lint. `lint_spanned` does not know what
surface it is on; the surface classification is Invariant 2's second
predicate, applied by Claudine and DMLS before they call in.

#### The rewrite generator

The suggestion is a complete rewritten expression, generated mechanically
from source text. Six rules govern it. Each exists because the obvious
version of the rewrite is wrong, and the failure modes are silent.

1. **Raw slicing, never the decoded value.** `read_string` decodes `\n`,
   `\t`, `\r`, `\\`, and the active quote, and that decoding is lossy: a
   source `\n` and a source raw newline both arrive as U+000A, so re-emitting
   from the decoded value would rewrite the author's text without saying so.
   `lex_spanned` sets the string token's span to include **both quotes**, and
   `parse_primary` carries it through, so `source[span.start + 1 ..
   span.end - 1]` is the raw, as-authored inner text. The generator slices
   that and never touches the decoded value.
2. **The original quote character is reused.** The lexer accepts both `'`
   and `"`. Every generated literal piece is re-wrapped in the quote
   character the original literal used, which keeps embedded quotes and
   escape sequences byte-identical to source and removes any need for a
   re-escaping pass.
3. **A lifted span that is not atomic is parenthesized.** `+` binds tighter
   than ternary, `||`, `&&`, and comparison, in the chain
   `parse_expression` → `parse_ternary` → `parse_ternary_branch` →
   logical-or → logical-and → comparison → `parse_additive` →
   multiplicative → unary → postfix → primary. Splicing a ternary in bare
   turns `"a {{ x ? y : z }} b"` into
   `"a " + x ? y : z + " b"`, which parses cleanly as
   `("a " + x) ? y : (z + " b")` — a different program that a round-trip
   parse check will not catch. Bare additive is unsafe for the same reason:
   `"a {{ x + y }}"` spliced flat is left-associative, so `x = 1`, `y = 2`
   gives `"a 12"` where the author meant `"a 3"`. Parentheses are in the
   grammar (`Token::LParen` in `parse_primary`, producing
   `SpannedExprKind::Paren`), so the rule is: wrap any lifted span that is
   not a `Variable`, `MemberAccess`, `Index`, `FunctionCall`, literal, or
   already-`Paren` in `( … )`.
4. **Every `+` in the chain must take the concatenation branch.** `+` is not
   unconditionally concatenation. `evaluate_binary` performs numeric
   addition when both operands are numbers, and also when one is a `Number`
   and the other a numeric `String` (`to_number_arithmetic`). Dropping the
   empty pieces of a literal is therefore wrong: `"{{a}}{{b}}"` becomes
   `a + b`, which is `3` rather than `"12"` for `a = 1`, `b = 2`, and
   `"{{x}}"` alone becomes `x`, which is no longer a string at all. The
   requirement is that no `+` the generator emits can reach the arithmetic
   branch.

   A single leading `""` anchor is necessary but **not** sufficient, because
   the accumulator itself can become a numeric string: `"" + a + b`
   evaluates as `("" + a) + b` = `"1" + 2`, and `"1"` is a numeric string
   meeting a `Number`, so the second `+` is arithmetic and the result is `3`
   again. The generator therefore anchors each lifted span individually, as
   `("" + <span>)`, except where a literal piece already to its left in the
   chain is non-empty and cannot itself parse as a number — in which case
   the accumulator is provably a non-numeric string and the bare span is
   safe. The incident's else branch takes the cheap path and reads
   naturally; `"{{a}}{{b}}"` and `"{{a}}5"` take the anchored path and are
   correct.
5. **A lifted span is decoded; the literal pieces are not.** The two halves
   of a rewrite want opposite treatments. In `"a {{ f(\"x\") }} b"` the
   pieces must stay raw (rule 1), but the lifted span is re-emitted as
   expression *source*, and raw `\"` would put a stray backslash in front of
   the lexer, which rejects it. The generator applies exactly the decoding
   `read_string` would have applied to the lifted span's text and leaves
   every piece raw. The decoded result is valid expression source: a nested
   string literal inside the lifted text needs no escaping once it is no
   longer inside an outer literal.
6. **The generator validates its own output.** The result is re-parsed with
   `parse_spanned` (or `parse_condition_spanned` for the condition dialect),
   and `suggestion` is `None` if that fails. This is a safety valve, not the
   correctness argument — rule 3's mis-splice re-parses cleanly. Correctness
   is asserted in Testing by AST equivalence.

The rewrite substitutes the new literal into the full expression source at
the literal's span, so the suggestion is the complete corrected expression.
When any nested span fails to parse, `suggestion` is `None`; the diagnostic
still fires.

**Accepted limitation: array-valued spans are not equivalent.** A rewrite is
judged against what the author meant — what the same span would have
produced on a rescanning surface. For an array-valued span the two cannot be
made to agree. `+` stringifies through `scalar_string`, which renders
`["a","b"]` as the JSON text `["a","b"]`; interpolation renders through
`interpolation_output_string`, which produces `a\nb`. No amount of
parenthesization or anchoring changes this, because the divergence is in the
two rendering functions, not in the rewrite. `prompts/commit.md`'s
`ctx.dirty_package_areas` is exactly this case. The lint still fires and
still offers the rewrite; the non-equivalence is documented (D5) and
asserted as a known divergence in Testing rather than treated as a bug.

**Not flagged.** A `{{{ … }}}` literal escape inside a quoted string is not
a nested span, because `find_all_plain` does not recognize it as one. A
literal that contains a lone `{{` with no closing `}}` is not flagged
either; it is unusual but it is not this defect. Neither is a literal on a
rescanning surface (Invariant 1).

### D2. Claudine rejects the document at prepare time

Add `CompositionError::LifecycleNestedSpanInLiteral`:

```rust
LifecycleNestedSpanInLiteral {
    source_path: PathBuf,
    /// Dotted lifecycle key, e.g. `success.say`, `finalize.stack[0].when`.
    property: String,
    /// The authored literal, quotes included.
    literal: String,
    /// The nested span text, e.g. `{{ctx.repo_name}}`.
    nested: String,
    /// The complete rewritten expression from D1, when available.
    suggestion: Option<String>,
    /// Frontmatter line of the literal, for the highlight.
    line: Option<usize>,
}
```

A new validator, `validate_no_nested_spans_in_literals`, runs in the same
prepare-time pass as `validate_no_undefined_lifecycle_variables` and takes
the same `raw_frontmatter`, because it needs source text. It covers the
non-rescanning surfaces:

- whole-value frontmatter scalars, identified with `is_whole_value_span`;
- the seven events' communication fields (`LIFECYCLE_COMM_FIELDS`), by
  finding each `{{ … }}` span in the raw string and linting its inner text
  with `ParseMode::Interpolation`;
- every `when` / `until` / `while` predicate, linted whole with
  `ParseMode::Condition`;
- every stack action operand and `proxy … with` value that
  `iter_stack_expression_surfaces` reports today, using the raw YAML string
  rather than the parsed `Expr`, so that only authored literals are examined
  (Invariant 3).

A mixed frontmatter string (`"a {{ … }} b"`) is not covered: it rescans and
resolves correctly.

**Raw-source dependency.** `iter_stack_expression_surfaces` yields a
`LifecycleExpressionSurface` holding `expr: &'a Expr` — parsed trees, with
no raw YAML text attached. Reading raw source from those surfaces is
therefore not free. The implementation plan must choose between extending
that iterator to carry the raw source alongside the tree, and adding a
parallel raw-text walker over the same surfaces; the second desyncs more
easily and the first touches existing callers. This spec does not settle it
but requires the plan to say which, because the choice determines whether
the two walks can drift apart.

The first lint in `LifecycleSignal::ALL` order aborts composition, matching
the other validators. The error renders through the existing
`FrontmatterHighlight::Line` machinery with:

- header `CompositionError: nested interpolation inside a string literal`;
- body naming `property`, quoting `literal`, and stating that
  `{{ … }}` inside a quoted string is literal text that is never
  interpolated on this surface;
- the suggestion, when present, printed as an illustrative rewrite rather
  than a paste-ready YAML line. It is bare expression text (Invariant 5) and
  may contain either quote character, so no single YAML scalar style is safe
  for it — a double-quoted scalar breaks the moment the rewrite emits a `"`.
  The rendering must not imply the line can be pasted unchanged;
- a hint pointing at `{{{ … }}}` for the rare case where literal braces
  were intended.

**Where it runs.** In the shared prepare stage, so `compose`,
`inline-compose`, and dry-run reject identically. The `proxy` handoff runs
the target's prepare stage before launch today, so this validator rides
along and the incident document is rejected when `review.md` proxies to it,
not when the agent finishes.

`sequence` is covered by a different mechanism, and the difference matters.
Phase 1c (`claudine/cli/src/commands/wrap/sequence/phase1c.rs`,
`run_phase_1c_with_schema` → `run_phase_1c_attempt`) is called
unconditionally before execution and, for every step, calls the same
`jit::compose_step` execution uses, ending in the shared prepare stage. Any
`CompositionError` other than `MissingProperties` aborts the run. A
validator on the prepare stage is therefore already enforced against every
step, with no sequence-specific work.

The gap Phase 1c leaves is narrow: it composes the *sequence document* per
step. A step that references a separate prompt document (`task:
some-prompt.md`) composes that file only at its own turn
(`StepComposeContext::for_referenced_document`; the loop-step equivalents in
`iterate.rs`), and Phase 1c never opens those files. Close it with an
up-front **text-only** pre-scan in `phase1c.rs`: for each step whose
executable names a document, resolve the file reference where it resolves
without runtime state, read it, and run only this text lint over its raw
frontmatter. A step reference whose own text contains `{{ … }}` cannot be
resolved up front and falls through to being caught at its own turn, which
is Invariant 4's first exception.

If a sequence step or proxy target reaches launch without having passed this
validator, and it is not one of Invariant 4's two exceptions, that is a
defect in this fix, not an accepted gap.

### D3. DMLS diagnoses it and offers a quick fix

**Scan frontmatter strings.** Add a frontmatter interpolation inventory to
`dmls/src/overlay/expressions.rs`, alongside the body `interpolations`
helper: every `{{ … }}` span inside a plain string scalar of the
`FrontmatterAst`, with the span projected into document byte offsets. This
is the gap that made the editor blind to every lifecycle expression, and it
is a prerequisite for D3, not an optional extra. Unknown-identifier and
malformed-expression diagnostics naturally extend to these spans; that
widening is in scope only insofar as it falls out of the shared walk. Any
new false positive it introduces on lifecycle keys (for example `err`,
`timing`, `current`, which are Claudine late-binding globals unknown to
DMLS) must be suppressed by adding those roots to the known-root authority
`is_unknown_root`, not by skipping lifecycle keys.

**Where the diagnostic fires.** Only on whole-value frontmatter scalars,
classified with `is_whole_value_span` (Invariant 2). A document body
interpolation and a mixed frontmatter string are rescanning surfaces and are
never flagged, however the inventory reaches them. DMLS has no view of
Claudine's schema, so brace-less predicate surfaces (`when`, `until`,
`while`) remain invisible to the editor and are covered by D2 alone; see
Open Questions.

**Working in raw coordinates.** The editor scans the raw document slice and
stays in raw coordinates end to end. It never decodes the scalar and never
needs a decoded-to-raw offset map. Concretely: run the span recognizer over
`&document[value_span]` and add `value_span.start` to every offset. This is
correct by construction, because stripping per-line indentation shifts
braces and quotes without creating or destroying any, so the set of spans
found in raw text equals the set found in decoded text — Invariant 2 holds
set-wise with no extra machinery, and the *n*-th nested span in document
order is the same span in both coordinate systems. The lint itself still
runs on the parsed `SpannedExpr` (only the tree can say which text is inside
an authored literal); its results are aligned to raw ranges by that
ordinal correspondence.

This is deliberately not a general block-scalar decoder, and the background
explains why:

- Darkmatter's schema scanner already excludes block scalars on purpose:
  `schemas/simplified/source.rs` guards `locate_inline` with
  `is_block_scalar_header` and returns `Err(projection_error())`, and
  `darkmatter/lib/tests/meta_schema_phase6.rs` asserts that "block scalars
  are outside the frozen v1 source-aware grammar".
- `decode_scalar_at` (`yaml_scalar.rs`) dispatches on the first byte; `|`
  falls to the `_` arm and reaches `plain_scalar`, an identity map. It
  returns the *wrong string* — raw text including the `|-` indicator and
  every line's indentation — presented as decoded. The problem is not
  imprecise offsets, it is wrong content.
- Adding `value_span.start + decoded_offset` on top of that drifts by the
  content indent, once per line. For the incident, authored at an eight-space
  indent, that is −8 bytes at the first line and −27 bytes by the line
  carrying the first nested span: the diagnostic would underline the wrong
  text.
- No block-scalar decoder with position tracking exists anywhere in the
  workspace. `rlsp-yaml-parser` decodes correctly and exposes
  `ScalarStyle::Literal(Chomp)` / `Folded(Chomp)`, but its block lexer is
  `pub(super)` and offers no per-byte map. Building one is duplicated parser
  logic with drift risk; if it is ever genuinely needed it belongs upstream
  in the YAML parser, not hand-rolled in Darkmatter.

Two facts make raw-coordinate scanning cheap: `value_span` already covers
`|-` through the end of the block, and the correctly decoded value is
already on `FmEntry.scalar`.

**Capture `ScalarStyle` on `FmEntry`.** `classify()`
(`dmls/src/overlay/frontmatter.rs`) computes the scalar style and discards
it; retaining it is roughly five lines. This is the enabling change, because
it is what lets the editor tell a literal `|` block from a folded `>` block.

**Folded scalars.** A folded `>` scalar rewrites line breaks into spaces, so
raw and decoded content genuinely diverge and a range or an edit computed in
raw coordinates could describe text the evaluator never sees. Folded scalars
are therefore ranged on the whole scalar and get no quick fix. Precision is
promised for flow scalars and literal `|` blocks, which is the incident's
exact shape and effectively all lifecycle authoring.

**Suppress the suggestion when a flagged literal's raw slice contains a
newline.** A quoted string literal spanning a line break inside a block
would otherwise carry raw YAML indentation into the suggestion text.

**Diagnostic.** Add `code::EXPRESSION_NESTED_SPAN_IN_LITERAL =
"dm.expression.nested_span_in_literal"`. Severity `WARNING`, matching
`EXPRESSION_MALFORMED`. Source `darkmatter.frontmatter`. Range: the nested
span's `{{` through `}}`, in raw document coordinates. Message:

```text
`{{ctx.repo_name}}` inside a quoted string is literal text and is never
interpolated here; concatenate with `+` instead
```

The frontmatter producer (`diagnostics/frontmatter.rs::expression_diagnostics`)
calls `lint_spanned` on the `SpannedExpr` it already parses, so the rule runs
once per span with no second parse.

**Fix the unguarded decode.** `dmls/src/providers/frontmatter.rs` calls
`decode_scalar(raw)` with no block-scalar guard, unlike `source.rs`. An
`Expression`-typed schema property authored as `|-` therefore hands
`|-\n        {{…` straight to the expression parser today and produces a
spurious `EXPRESSION_MALFORMED` warning; offsets stay correct, the text does
not. Adding the guard is in scope here because this fix is what puts block
scalars on the editor's expression path in earnest.

**Code action.** In `providers/code_actions.rs`, a `quickfix` titled
`Rewrite with + concatenation` for every diagnostic carrying this code whose
lint produced a suggestion. The edit replaces the offending `{{ … }}` span's
inner text with the suggestion. The `|-` indicator and the per-line
indentation survive untouched because the edit is computed in raw
coordinates and never reconstructs the scalar — not because the edit takes
care to preserve them.

**Hover.** No change. The existing hover on a string literal already shows
its value; that value will visibly contain the braces, which is a hint on
its own.

**Implementation-start check.** Confirm that the expression lexer tolerates
raw YAML indentation embedded in expression text. This is expected to hold —
the incident's own ternary parses from decoded text that already contains
newlines and four-space indentation — and is recorded as a check to run, not
as an unknown.

### D4. The event-time message stops sounding like the crash

The runtime guard stays, because a frontmatter value can still hold raw
template text that no static pass can see. Its report changes:

- `LifecycleEvaluationError` is a variant of `CompositionError`, and its
  `surface` field is a plain `String`. It gains the lifecycle key, so the
  renderer prints `in success.say` instead of `in an interpolated string`.
- The `reject_surviving_spans` reason becomes:

  ```text
  the rendered text still contains `{{ctx.repo_name}}` after every
  interpolation pass; a `{{ … }}` inside a quoted string literal is text
  and is never interpolated on this surface, and a frontmatter value that
  holds template syntax is not re-expanded at event time
  ```

- The hint for this reason is specific: concatenate with `+`, or use
  `{{{ … }}}` for intentional braces. The generic "resolve the missing path
  or variable" hint is reserved for evaluation errors that are actually
  about missing paths or variables. The renderer selects the hint from a
  typed reason, not by matching message text — which means introducing a
  typed reason where a `String` sits today. That is the sizeable part of
  D4: the variant, its construction sites, and the renderer all move
  together.

### D5. Documentation

- `claudine/docs/topics/lifecycle.md`: a new `LifecycleNestedSpanInLiteral`
  entry under Validation, with the incident's before and after, and a
  sentence under "Action Forms" stating that braces inside a quoted
  expression literal are inert on a lifecycle surface. The claim that
  `validate_no_interpolation_leaks` "still runs" for non-lifecycle surfaces
  is corrected to match whatever Open Question 4 decides.
- `darkmatter/docs/inline/interpolation.md`: a "Braces inside string
  literals" rule next to the `{{{ … }}}` recognition rules, stating which
  surfaces rescan and which do not, that a quoted literal is never
  re-scanned within a single pass, and showing the `+` form. The
  array-rendering divergence between `scalar_string` and
  `interpolation_output_string` is stated here, because it is the reason the
  rewrite is not a semantic identity.
- `darkmatter/docs/lsp/features.md` section 4.3: a row for the new
  diagnostic and quick fix, including the folded-scalar limitation; section
  4.1: a note that frontmatter string scalars are now scanned for
  interpolation spans.
- `.claude/skills/claudine/` and `.claude/skills/darkmatter/` lifecycle and
  expression authoring guidance: the rule and the rewrite, one paragraph
  each. Hash updates via `md hash`.
- `claudine/README.md` and `darkmatter/README.md` only if they enumerate
  validation errors or diagnostic codes today.

## Scope

In scope:

- D1 through D5 above.
- The DMLS frontmatter-string scan and the `ScalarStyle` capture on
  `FmEntry` (D3), because without them the editor half of the outcome is
  unreachable.
- The missing block-scalar guard on `decode_scalar` in
  `dmls/src/providers/frontmatter.rs` (D3).
- Correcting both live instances. This is an outstanding task, not a
  completed one:
    - `prompts/_reviews/review-spec-inline.md` was partially edited by hand
      and is currently worse than before. `success.say` reads
      `… + ctx.area + "" has completed"`, a stray `"` that makes the
      expression malformed, and `failure.say` still contains
      `{{ctx.area}}` inside a literal. Acceptance Criterion 3 fails today.
    - `prompts/commit.md`'s `resides_in`.
- Two regression fixtures, captured from the pre-fix state of those two
  files: the incident document, and the `commit.md` `resides_in` value with
  its single quotes, `\n` escape, two nested spans in one literal, and
  array-valued span.

Out of scope:

- Making string literals interpolate. Invariant 1 rules it out; the `+`
  operator and ternaries already express every case.
- Changing the rescan behavior of `interpolate_text`, or extending the rule
  to rescanning surfaces.
- A general expression linter framework beyond the one-variant enum in D1.
  The enum exists so a second rule can be added without a new API, not as
  an invitation to add one now.
- Changing the brace-depth behavior of the span finder.
- A general block-scalar decoder with a decoded-to-raw offset map.
- An `md lint` or `md validate` CLI surface for expressions. `md validate`
  is schema validation today; adding expression lints there is a separate
  decision (see Open Questions).

## Testing

Every new test must fail on the commit before its fix, and the verification
record must say how that was shown.

### Darkmatter L1 (`darkmatter/lib`)

- `lint_expression` fires once per nested span for double- and single-quoted
  literals, in both parse modes, in ternary branches, function arguments,
  array elements, and object-literal keys and values.
- It does not fire for `"a " + b + " c"`, for `{{{ x }}}` inside a literal,
  for an unclosed `{{` inside a literal, or for any expression with no
  literals.
- **Negative case, rescanning surface.** `{{ pkg ? 'in {{pkg}}' : 'x' }}` in
  a document body still composes to the interpolated text, produces no lint,
  and is not reported by any caller.
  `rescans_replacement_text_for_nested_interpolation` and
  `rescans_false_branch_for_nested_interpolation` stay green and unmodified.
- The suggestion for the incident's else branch is exactly
  `"The review of the draft specification file in the " + ctx.repo_name + " repo has completed"`
  substituted into the full ternary.
- **Round-trip is asserted by AST equivalence**, not by re-parsing. The
  rewritten expression's tree must be equivalent to the tree the author
  meant; re-parsing alone passes on the ternary mis-splice in D1 rule 3 and
  is therefore not a test.
- Rewrite generator cases, each asserted on evaluated output:
    - ternary splicing — `"a {{ x ? y : z }} b"` does not become
      `("a " + x) ? y : (z + " b")`;
    - numeric `+` — `"a {{ x + y }}"` with `x = 1`, `y = 2` evaluates to
      `"a 3"`, not `"a 12"`;
    - adjacent spans — `"{{a}}{{b}}"` with `a = 1`, `b = 2` evaluates to
      `"12"`, not `3`; `"{{a}}5"` evaluates to `"15"`, not `6`; `"{{x}}"`
      alone stays a string;
    - escaped quotes — `"a {{ f(\"x\") }} b"` rewrites with the pieces raw
      and the lifted span decoded, and the result parses;
    - single-quoted literals keep single quotes, and an embedded `\n` escape
      survives byte-identically (the `commit.md` fixture).
- **Array divergence is asserted, not fixed.** For an array-valued span the
  rewritten expression yields `scalar_string` JSON while the rescanning
  interpolation yields newline-joined text. The test pins both values so the
  divergence is visible and cannot be "fixed" by accident.
- A literal whose nested span is malformed yields a lint with no suggestion.
- Property: for any expression source, the set of literals `lint_spanned`
  flags equals the set of literals on which `find_all_plain` is non-empty.
  This is Invariant 2's first half as a test.
- `is_whole_value_span` agrees with the branch `interpolate_value` actually
  takes, over the same inputs. This is Invariant 2's second half as a test.

### Claudine L1 (`claudine/lib`, `claudine/cli`)

- `validate_no_nested_spans_in_literals` rejects the incident frontmatter at
  `success.say` with the expected literal, nested text, and suggestion, and
  reports `failure.say` when `success.say` is fixed.
- It rejects a nested span in a `when` predicate, in a positional `message`
  operand written as `"{{ cond ? 'a {{x}}' : 'b' }}"`, and in a
  `proxy … with` value.
- It does **not** reject a positional action body such as
  `info: "running {{agent}}"`, a key/value `message: "Deployed {{version}}"`,
  a `set_frontmatter: ["s.md", "k", "{{ payload }}"]`, or a mixed
  frontmatter string `"a {{ x ? 'in {{x}}' : 'y' }} b"`. The first three are
  synthesized literals; the last is a rescanning surface.
- CLI process test in `wrap_compose_validation.rs`, using the stub-provider
  pattern already there: composing the incident prompt exits non-zero, names
  `success.say` and the literal on stderr, prints the rewrite, and the stub
  provider is never executed. The stub records an invocation marker so
  "never executed" is asserted, not assumed.
- The same through a `proxy` handoff: an entry prompt that proxies to the
  incident prompt fails before the stub runs.
- The same through `sequence` with the incident prompt as step two: step
  one's stub never runs either, because validation of every step precedes
  the first launch.
- The Phase 1c pre-scan opens a step's **referenced** prompt document and
  rejects it before step one launches, where the reference is a literal
  path. Where the reference is itself interpolated, the run starts and the
  defect is caught at that step's turn, still before that step's provider is
  spawned.
- D4: a frontmatter value holding raw template text reaches the runtime
  guard, and the stderr names the lifecycle key and carries the new reason
  and hint. The old hint text must not appear.

### DMLS L1 (`darkmatter/dmls`)

- Opening the incident file produces two
  `dm.expression.nested_span_in_literal` warnings, one per `say`, each
  ranged on the inner `{{ … }}` inside the block scalar, with the source
  `darkmatter.frontmatter`.
- A body interpolation with the same defect produces **no** warning of this
  code, and the document still composes.
- The code action rewrites the block scalar's expression text and leaves
  the `|-` indicator and indentation untouched; the resulting document
  produces zero diagnostics of this code.
- A folded (`>`) scalar carrying the defect is ranged on the whole scalar
  and offers no quick fix.
- An `Expression`-typed schema property authored as a `|-` block no longer
  produces a spurious `EXPRESSION_MALFORMED` warning.
- Frontmatter-string scanning does not introduce `EXPRESSION_UNKNOWN_IDENTIFIER`
  on `err`, `timing`, or `current` in lifecycle keys.
- The `no_side_effects.rs` and `packaging_contract.rs` suites stay green.

### Evidence

macOS L1 for all three areas locally through `just test`. Linux, native
Windows, and WSL2 through CI on the PR. The DMLS L2 editor suite is not
required for this change. The verification record lists the failing-before
proof for each new test group.

## Acceptance Criteria

1. Composing the pre-fix `review-spec-inline.md` fixture with any provider
   exits non-zero at prepare time, names `success.say`, quotes the literal,
   prints the `+` rewrite, and spawns no provider process.
2. The same document proxied to, or run as any step of a sequence, is
   rejected before any provider in that run is spawned. Where a sequence
   step's document reference is itself interpolated, rejection happens at
   that step's turn, before that step's provider is spawned.
3. `prompts/_reviews/review-spec-inline.md` and `prompts/commit.md` are
   corrected, compose cleanly, fire their lifecycle events with no error,
   and the spoken and written text contains no braces. This is outstanding
   work: the current `review-spec-inline.md` is malformed and would fail
   this criterion today.
4. Every positional action body and key/value parameter that interpolates
   today still interpolates, and every rescanning surface that resolves a
   nested span today still resolves it. The claudine L1 suite and the two
   `rewrite.rs` rescan tests are unchanged and green.
5. `lint_expression` and `reject_surviving_spans` share one span
   recognizer; Claudine, DMLS, and the lint share one surface classifier;
   and both property tests in the Darkmatter L1 suite pass.
6. DMLS emits `dm.expression.nested_span_in_literal` for whole-value
   frontmatter scalars with the ranges and source in D3, emits nothing for
   body spans or mixed frontmatter strings, and the quick fix produces a
   document with zero such diagnostics.
7. DMLS scans `{{ … }}` spans in plain frontmatter string scalars, and
   Claudine's late-binding globals do not produce unknown-identifier noise.
8. The event-time guard's message names the lifecycle key and the new
   reason, selected from a typed reason rather than by message matching;
   the "resolve the missing path or variable" hint no longer appears for a
   surviving span.
9. Docs listed in D5 are updated in the same change, and skill hashes are
   refreshed with `md hash`.
10. L1 green on macOS locally and on Linux, Windows, and WSL2 in CI.

## Open Questions

1. **Should `md validate` grow an expression-lint pass?** It would give a
   CLI surface outside Claudine and outside an editor. Options: (a) add
   `--expressions` to `md validate` that runs `lint_expression` over body and
   frontmatter spans; (b) a new `md lint`; (c) leave it to DMLS and Claudine.
   Recommendation: (c) for this fix. Claudine already rejects at prepare
   time and DMLS covers authoring. Revisit when a second lint rule exists.
2. **Severity in DMLS: warning or error?** The construction can never be
   correct on a non-rescanning surface, which argues for `ERROR`. The
   consistency argument that was offered for `WARNING` was based on a false
   premise and does not survive checking: DMLS does **not** render every
   expression problem as `WARNING`. `EXPRESSION_MALFORMED` is `WARNING`
   (`providers/dsl.rs`, `diagnostics/frontmatter.rs`), but
   `EXPRESSION_UNKNOWN_IDENTIFIER` is `DiagnosticSeverity::INFORMATION` in
   both producers. There is already a severity gradient, so the question is
   which rung this rule belongs on, not whether the ladder is flat. Still
   unruled; D3 specifies `WARNING` provisionally.
3. **Should the `err` / `timing` / `current` roots be known to DMLS in every
   document, or only under lifecycle keys?** Recommendation: everywhere, via
   the shared known-root authority. Scoping to lifecycle keys would make
   DMLS reason about Claudine's schema, which it does not do today.
4. **What happens to `validate_no_interpolation_leaks`?** It already exists,
   walks `iter_stack_expression_surfaces` with `visit_string_literals`
   looking for surviving spans in literals, is exported from
   `lifecycle/mod.rs`, and has tests — and no production call site was found
   for it. The new validator overlaps it heavily. The plan must decide
   whether `validate_no_nested_spans_in_literals` absorbs it, replaces it,
   or sits beside it, and must not leave a second uncalled validator with
   green tests behind. Whichever way it goes,
   `claudine/docs/topics/lifecycle.md`'s claim that it "still runs" for
   non-lifecycle surfaces is corrected in the same change.
5. **Does the editor need to see `when` / `until` / `while` predicates?**
   Those surfaces carry bare condition text with no `{{ … }}` wrapper, so
   the D3 frontmatter inventory cannot find them and only Claudine's schema
   knows they are expressions. D2 covers them at prepare time, so the defect
   is always caught — but the editor stays silent on a surface the rule
   applies to, which makes the "three places" promise in Outcome partial.
   Options: (a) accept it, and say so in the docs; (b) let DMLS learn the
   predicate keys, which is exactly the Claudine-schema knowledge Open
   Question 3 declines to give it; (c) require predicates to be written as
   `{{ … }}` spans, a language change well outside this fix. Not ruled.

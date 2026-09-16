---
created: 2026-09-13
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-15
implemented: false
clarified: true
needs_rulings: false
clarified_by: claude/opus
area: claudine
packages:
    - darkmatter
    - dmls
    - claudine
    - claudine-cli
related:
    - darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md
    - claudine/fixes/_completed/2026-09-05-inline-flow-and-validations/spec.md
    - claudine/docs/topics/lifecycle.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/lsp/features.md
references:
    claudine/fixes/2026-09-13-better-static-analysis/spikes/entry-arena-cost.md: >-
        Measured entry counts, lookup timings, and schema-shape clone counts
        for D6's sequence descent across 2,441 documents. Read it before
        touching the entry arena: it is the evidence that the descent needs
        no pointer index, the reason the `nested_shape_for_completion` memo
        is mandatory, and the source of the item-entries-vs-keys-only
        decision.
    claudine/fixes/2026-09-13-better-static-analysis/spikes/parser-agreement.md: >-
        Establishes that DMLS reuses Darkmatter's parser verbatim and that
        the two dialects accept identical strings, so D7's `ERROR` raise
        cannot outrank the composer. Also the source of the widened decode
        guard, the wrong-range-on-a-true-positive correction, and the
        `union_rejected` invariant.
    darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md: >-
        The companion fix. Unifies Darkmatter's two array renderings on JSON
        and lifts D1's suggestion suppression. Read it to understand why the
        suppression exists and when it goes away.
    darkmatter/fixes/2026-08-27-preserve-backslash-escapes/spec.md: >-
        Merged as bdf471489. Establishes that compose must not consume
        backslash escapes in body text, which is the invariant D8's escape
        mechanism is built to respect rather than violate.
    darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md: >-
        Its D4 introduced the list-rendering function family and chose
        newline-joining as the bare-array default. The companion fix reverses
        that choice; this one depends on the family existing.
    darkmatter/docs/schemas/darkmatter.yaml: >-
        The authored base schema. Declares the type of every `ctx.*`
        variable, which is what makes D1's suggestion suppression
        type-directed rather than guesswork.
    darkmatter/lib/src/markdown/compose/context/catalog.rs: >-
        Projects the base schema's `ctx` block into descriptors carrying
        `ContextValueType { base, is_array, integer }`. The lookup D1's
        suppression rule consults.
---

# Make Expression Defects Visible Before Anything Runs

## Outcome

This began as one defect — a `{{ … }}` span written inside a quoted string
literal, which ran an agent to completion and then crashed. Investigating it
turned up three more ways the expression layer fails the author: it does not
look where authors write, it does not complain at a volume that matches how
wrong a thing is, and it gives an author no way to say "this text is not for
you". All four ship together, because the rule is not worth much if it
cannot reach the surfaces people use, is easy to ignore, or cannot be
switched off where it does not belong:

- **The nested-span rule** (D1–D5). The original defect: a nested span is
  rejected in the editor, at prepare time, and — for what static analysis
  cannot see — at event time.
- **Full frontmatter coverage** (D6). The editor's frontmatter model never
  descends into list items, so roughly a third of real authoring surface is
  dark to it. The descent is what makes the rule — and every other
  frontmatter diagnostic — reach the surfaces authors actually use.
- **A severity ladder** (D7). Warnings mean "might be wrong"; errors mean
  "will never work". Applied consistently, which moves two existing
  diagnostics as well as placing the new one, and which separates a
  schema-declared expression from a brace pattern merely inferred to be one.
- **A backslash escape** (D8). `\{{` opts a span out of being scanned at
  all, so a document that discusses another template language stops being
  diagnosed as though it were written in this one.

A fourth change found during this work — unifying Darkmatter's two array
renderings on JSON — is specified separately in
[`darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md`](../../../darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md).
It changes rendered output rather than diagnostics, and it reverses a
shipped decision, so it gets its own revert boundary. D1 below records the
one place the two fixes touch.

> **Reader's note (2026-09-15 review):** This revision separates the
> syntactic whole-value shape from the semantic single-pass contract. An
> ordinary frontmatter key can receive a second interpolation pass after
> frontmatter shell expansion; a lifecycle field cannot. Therefore the hard
> diagnostic is limited to surfaces whose owning contract guarantees one
> evaluation pass. The review also makes schema-directed suggestion filtering
> an explicit lint API, defines lossless YAML-style projection for DMLS fixes,
> gives sequence-item arena entries an explicit role, and specifies odd/even
> backslash handling. Those details prevent the implementation from making
> correctness depend on an unrelated shell expansion, corrupting YAML while
> applying a quick fix, inventing a mapping key for a list item, or treating a
> literal backslash as an escape marker.

The original defect's outcome is unchanged and remains the motivating case.
An author who writes a `{{ … }}` span inside a quoted string literal of a
Darkmatter expression, on a surface that is evaluated once, finds out in
three places, in this order, and never after an agent has already run:

1. **In the editor.** DMLS underlines the inner braces with an error that
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
`fixes/_complete/2026-09-13-cicd-redundancies/spec.md`. The review ran to completion
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
coverage and are why this surface looked guarded. Its disposition is a
Resolved Implementation Determination: fold its valid cases into the new
raw-aware validator and runtime guard, then delete the dead entry point.

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
  expression. Surfaces fall into three tiers, and the whole of this fix turns
  on the division:
    - **Rescanning surfaces** are evaluated through `interpolate_text`, which
      loops to a fixpoint over its own output: a document body
      interpolation, and a mixed frontmatter string such as
      `"a {{ … }} b"`. A nested span on one of these is resolved by the next
      depth pass.
    - **Conditionally rescanned surfaces** use the one-pass
      `whole_value_span` branch for each call but can be revisited by an owning
      pipeline. Ordinary frontmatter is the important case: a key may receive
      a second interpolation pass after frontmatter shell expansion. The
      second pass is conditional on shell replacement, so authors must not
      rely on it, but its existence means the construct is not truthfully a
      “will never work” error on an arbitrary frontmatter key.
    - **Non-rescanning surfaces** are contractually evaluated exactly once:
      a whole-value lifecycle communication field, every lifecycle `when` /
      `until` / `while` predicate, every stack action operand, and every
      `proxy … with` value. These are the only surfaces on which this fix emits
      the hard nested-span diagnostic.

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
2. **One recognizer; do not mistake syntax for execution policy.** The static
   rule and `reject_surviving_spans` decide “is this a span” with the same
   function. Darkmatter exports `is_whole_value_span` as the sole authority
   for the syntactic shape that `interpolate_value` branches on. It does
   **not** claim that the owning pipeline makes only one call: ordinary
   frontmatter disproves that when shell expansion triggers pass 2. Claudine's
   canonical lifecycle-surface iterator owns the single-pass inventory. DMLS
   mirrors only the lifecycle-key portion of that inventory until schema
   triggers replace its documented static key list (D3). Claudine and DMLS
   each test their inventory against the authored
   `darkmatter/docs/schemas/claudine.yaml` event keys, so a new lifecycle event
   cannot silently become editor-dark without introducing a crate dependency
   in either direction.
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
5. **Editor and CLI agree.** On every lifecycle surface both understand, the
   DMLS diagnostic range is the inner `{{`
   through its `}}`, and the Claudine error highlights the same frontmatter
   value and quotes the same literal. The suggestion is the **bare
   expression text** — no `{{ }}` wrapper, no YAML quoting — so the two
   consumers can assert byte equality on it; each is responsible for its own
   wrapping. Precise inner-brace ranges are promised for untagged plain,
   single-quoted, and double-quoted scalars plus literal `|` blocks; a folded
   `>` scalar is ranged on the whole scalar (D3).

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

pub enum DeclaredExpressionValueKind {
    Scalar,
    Array,
    Object,
    Unknown,
}

pub fn lint_expression_with_types(
    source: &str,
    mode: ParseMode,
    classify: impl FnMut(&SpannedExpr) -> DeclaredExpressionValueKind,
) -> Vec<ExpressionLint>;

pub fn lint_spanned_with_types(
    source: &str,
    expr: &SpannedExpr,
    classify: impl FnMut(&SpannedExpr) -> DeclaredExpressionValueKind,
) -> Vec<ExpressionLint>;

/// True when the trimmed text is exactly one `{{ … }}` span: the shape
/// `interpolate_value` routes to `whole_value_span`. This classifies syntax
/// only; it does not claim how many times an owning pipeline invokes it.
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

The untyped entries apply the documented default that an unknown value is
treated as scalar for suggestion purposes. The `_with_types` entries are the
required path for Claudine and DMLS: the generator parses each lifted nested
span, passes that AST to `classify`, and applies the suppression policy below.
This API boundary is necessary. A `lint_spanned(source, expr)` function with no
schema or catalog input cannot truthfully promise schema-directed filtering,
and duplicating the filter in each consumer would violate the editor/CLI
agreement invariant.

Callers decide *whether* to run the lint. The lint does not know what surface
it is on; Claudine applies its lifecycle inventory and DMLS applies the
schema/lifecycle inventory described in D3 before calling it.

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
   requirement is that **no `+` the generator emits can reach the arithmetic
   branch**.

   A single leading `""` anchor is necessary but not sufficient, because the
   accumulator itself becomes a numeric string: `"" + a + b` associates as
   `("" + a) + b` = `"1" + 2`, and a numeric `String` meeting a `Number` is
   exactly the arithmetic case, so the result is `3` again. Both halves are
   pinned by existing tests —
   `mixed_numeric_string_and_number_addition_is_arithmetic` and
   `two_numeric_strings_still_concatenate`
   (`darkmatter/lib/src/markdown/compose/expression/mod.rs`).

   The rule is therefore a **per-span anchor**: each lifted span is emitted
   as `("" + <span>)`, relaxed to a bare span where a non-empty literal
   piece already sits to its left in the chain and that piece cannot itself
   parse as a number, which makes the accumulator provably a non-numeric
   string. The incident's else branch takes the relaxed path and reads
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

**Array-valued spans: the suggestion is suppressed, by declared type.** A
rewrite is judged against what the author meant — what the same span would
have produced on a rescanning surface. For an array-valued span the two
paths render differently today: `+` stringifies through `scalar_string`
(JSON), interpolation through `interpolation_output_string`
(newline-joined). The divergence is in the two rendering functions, not in
the rewrite, so no anchoring or parenthesization touches it.

The suppression is **type-directed**, not guesswork. Darkmatter declares a
type for every context variable in `darkmatter/docs/schemas/darkmatter.yaml`,
and `darkmatter/lib/src/markdown/compose/context/catalog.rs` projects that
YAML into a descriptor carrying `ContextValueType { base, is_array,
integer }` — the catalog is generated from the schema rather than
hand-declared, so the two cannot drift. Frontmatter variables carry their
type the same way, through the document's own `$schema`. The rule:

- **Declared array** — suppress. `is_array` is the whole test.
- **Declared object** — offer the suggestion. Both existing rendering paths
  already serialize objects as JSON, so suppressing objects would withhold a
  correct fix without protecting semantics.
- **Declared scalar** — offer the suggestion.
- **Untyped** — offer the suggestion. A user-defined variable *can* be typed
  if the author wants it checked; anything untyped is effectively `any`, and
  the right default there is to trust the author.

This makes suggestion quality a function of how well a document is typed,
which is the same direction the architectural note at the end of D3 points:
the more the schema knows, the less the tooling has to guess. It also
catches the case that actually bit — `ctx.dirty_package_areas` is declared
`string[](generated; required)`, so `prompts/commit.md`'s literal is
suppressed rather than given an unfaithful rewrite.

The interim risk is bounded and worth stating rather than leaving to be
discovered: an **untyped** variable that happens to hold an array can
receive an imperfect suggestion until the array-rendering fix lands. A typed
one cannot. The window closes entirely when
[`darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md`](../../../darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md)
makes both paths render arrays as JSON and **lifts this suppression as part
of its own scope**. If that fix lands first, callers classify arrays normally
and the `_with_types` entries can be omitted if no other lint needs them; do
not add type plumbing solely to delete it in the same landing sequence.
Neither fix blocks the other.

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

- the seven events' whole-value communication fields
  (`LIFECYCLE_COMM_FIELDS`), first classified with
  `is_whole_value_span`, then linted with `ParseMode::Interpolation`;
- every `when` / `until` / `while` predicate, linted whole with
  `ParseMode::Condition`;
- every stack action operand and `proxy … with` value that
  `iter_stack_expression_surfaces` reports today, using the raw YAML string
  rather than the parsed `Expr`, so that only authored literals are examined
  (Invariant 3).

A mixed lifecycle string (`"a {{ … }} b"`) is not covered: it rescans and
resolves correctly. Neither is an arbitrary whole-value frontmatter key. The
latter is a conditionally rescanned surface, not part of the lifecycle
single-pass inventory, and rejecting it as an `ERROR` would overstate what the
runtime guarantees.

**Raw-source dependency.** `iter_stack_expression_surfaces` yields a
`LifecycleExpressionSurface` holding `expr: &'a Expr` — parsed trees, with
no raw YAML text attached. Reading raw source from those surfaces is
therefore not free. Build one `LifecycleSourceMap` from raw frontmatter and
extend the canonical iterator's records with the matching authored
scalar/span. Do not add a parallel surface walker. A configured parsed
surface with no source-map record is an internal validation error rather
than a silent skip (Resolved Implementation Determination 2).

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
up-front **passive, validation-only** pre-scan in `phase1c.rs`: for each step
whose executable names a document, resolve the authored reference through
`biscuit_file::FileReference` using the same captured
`FileResolutionContext` and `for_source` derivation as execution, read it, and
run only the YAML-aware lifecycle lint over its raw frontmatter. This pass does
not compose values, execute shell, fetch remotes, prompt for schema values, or
mutate the document. Calling it “text-only” is insufficient because the lint
must distinguish whole-value from mixed scalars and lifecycle paths from
ordinary keys. A step reference whose own text contains `{{ … }}` cannot be
resolved up front and falls through to being caught at its own turn, which is
Invariant 4's first exception. No prefix checks, ambient `resolve()`, or second
file-reference grammar is permitted on this path.

If a sequence step or proxy target reaches launch without having passed this
validator, and it is not one of Invariant 4's two exceptions, that is a
defect in this fix, not an accepted gap.

### D3. DMLS diagnoses it and offers a quick fix

**Scan frontmatter strings.** Add a frontmatter interpolation inventory to
`dmls/src/overlay/expressions.rs`, alongside the body `interpolations`
helper: every `{{ … }}` span inside a string-valued scalar of the
`FrontmatterAst`, with the span projected into document byte offsets. This
is the gap that made the editor blind to every lifecycle expression, and it
is a prerequisite for D3, not an optional extra. Unknown-identifier and
malformed-expression diagnostics naturally extend to these spans; that
widening is in scope only insofar as it falls out of the shared walk.

**Late-binding roots are recognized only under lifecycle keys.** `err`,
`timing`, and `current` are Claudine globals that exist only while a
lifecycle event is firing. Adding them to the known-root authority
`is_unknown_root` unconditionally would silence a genuine unknown-identifier
report everywhere else in the document, so they are scoped: recognized under
lifecycle keys, unknown elsewhere.

This is a knowing trade. It requires DMLS to hold a static list of which
keys are lifecycle keys — the Claudine-schema coupling this spec otherwise
avoids — and that list will drift as lifecycle keys change. The drift is the
accepted cost of scoping the roots correctly until the mechanism that
removes the need exists; see the architectural note at the end of D3.

**Where the diagnostic fires.** Only on whole-value scalars underneath the
static lifecycle-key inventory, classified with `is_whole_value_span`, and on
schema-typed lifecycle predicate values, which are condition text rather than
a span and so reach the lint by the other route below. An arbitrary
whole-value frontmatter key is not enough: its owning pipeline may invoke a
second pass after shell expansion. A document body interpolation, a mixed
frontmatter string, and a non-lifecycle whole-value key are never flagged,
although the inventory reaches them for the existing lower-severity
diagnostics. The path matcher is tested against the authored Claudine schema's
event keys and representative stack paths; Claudine separately tests
`LifecycleSignal::ALL` against that same schema authority, so the stopgap list
cannot drift silently.

**What the editor could not reach before D6.** The frontmatter AST is built
by `lower_mapping` (`dmls/src/overlay/frontmatter.rs`), which recurses into
`Node::Mapping` only. A sequence value produces one entry and the walk
stops, so nothing inside a `- item` line has an `FmEntry` at all. Every
scalar nested in a sequence is therefore invisible to the inventory,
regardless of schema typing — which covers stack action operands,
`proxy … with` values, and the `when` / `until` / `while` predicates that
live inside `stack` items — about a third of the expression-bearing lines in
`prompts/`. **D6 removes this limit** by teaching the overlay to descend
into sequences, and D3 depends on it.

**Predicates are reached through the schema, not a key list.** `when`,
`until`, and `while` carry bare condition text with no `{{ … }}` wrapper, so
the brace-scanning inventory cannot find them however deeply it walks. They
are instead declared **expression-typed in the schema**, which routes them
through the editor's existing schema-driven path (`expression_values` →
`diagnostics/frontmatter.rs::expression_diagnostics`). Two facts make that
work with no new machinery: the path already parses with the condition
dialect — it calls `overlay::expressions::parse_condition`, not the
interpolation parser — so predicates are exactly the dialect it was built
for; and D6's array-element step in `nested_shape` is what lets a path like
`initialize.stack[0].when` resolve at all. The schema carries the knowledge;
the editor learns no Claudine layout.

**Projection is scalar-style aware.** There is no correct single rule that
stays in raw coordinates for every YAML scalar. Flow quotes and escapes must
be decoded before expression parsing; literal blocks retain raw line breaks
but add a YAML header and indentation; folded blocks change the expression
text itself. The inventory therefore carries both semantic expression text
and a projection policy:

- **Untagged plain scalars:** parse the authored expression text directly and add
  `value_span.start` to lint spans.
- **Untagged single- and double-quoted scalars:** use the existing
  `DecodedScalar` map. Parse decoded text, project diagnostic spans back to
  authored bytes, and YAML-escape a replacement fragment for the original
  scalar style before constructing a code action.
- **Literal `|` blocks:** find each outer `{{ … }}` in the raw value slice and
  parse that raw inner expression. YAML indentation is ordinary expression
  whitespace, so the lint and its complete suggestion stay in authored
  coordinates and preserve every untouched byte.
- **Folded `>` blocks:** parse `FmEntry.scalar`, which is the parser-decoded
  value, but range the whole scalar and offer no code action because no
  decoded-to-authored map exists.
- **Tagged scalars:** analyze the decoded `FmEntry.scalar` only when needed
  for a diagnostic, range the whole scalar, and offer no action. **Aliases are
  not scalar entries and do not produce expression diagnostics at the alias
  site.** Their target remains the schema validator's responsibility.

Ordinal matching between a decoded tree and a separately scanned raw scalar
is not an accepted projection mechanism: repeated spans, YAML escapes, and
folding make “the nth span” too weak a source-position contract.

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

The correctly decoded value is already on `FmEntry.scalar`; the new work is
choosing the safe projection policy, not implementing another YAML decoder.

**Capture `ScalarStyle` and tag presence on `FmEntry`.** `classify()`
(`dmls/src/overlay/frontmatter.rs`) matches a `Node::Scalar` that already
carries both and currently discards them; retaining the style plus
`tagged: bool` is the enabling change. It lets the editor distinguish literal
from folded blocks and prevents a tagged scalar from entering an offset map
that assumes the first byte is the scalar itself.

**Folded scalars.** A folded `>` scalar rewrites line breaks into spaces, so
raw and decoded content genuinely diverge and a range or an edit computed in
raw coordinates could describe text the evaluator never sees. Folded scalars
are therefore ranged on the whole scalar and get no quick fix. Precision is
promised for untagged plain/single/double scalars and literal `|` blocks,
which is the incident's exact shape and effectively all lifecycle authoring.

**Suppress the suggestion when a flagged literal's raw slice contains a
newline.** A quoted string literal spanning a line break inside a block
would otherwise carry raw YAML indentation into the suggestion text.

**Diagnostic.** Add `code::EXPRESSION_NESTED_SPAN_IN_LITERAL =
"dm.expression.nested_span_in_literal"`. Severity **`ERROR`** under D7's
ladder: on the surfaces it fires on the construct can never work, and
compose refuses the document. Source `darkmatter.frontmatter`. Range: the
nested span's `{{` through `}}`, in raw document coordinates. Message:

```text
`{{ctx.repo_name}}` inside a quoted string is literal text and is never
interpolated here; concatenate with `+` instead
```

The frontmatter producer (`diagnostics/frontmatter.rs::expression_diagnostics`)
calls `lint_spanned_with_types` on the `SpannedExpr` it already parses when the
schema-driven path and interpolation inventory refer to the same projected
source. Literal-block outer spans are parsed once from their raw inner text;
folded/tagged values are parsed once from `FmEntry.scalar`. No diagnostic path
parses the same expression twice merely to obtain a lint.

**Fix the unguarded decode.** `dmls/src/providers/frontmatter.rs` calls
`decode_scalar(raw)` on styles it does not decode. The existing malformed-
expression producer is limited to untagged plain, single-quoted, and
double-quoted scalars, where `DecodedScalar` provides an exact map. Literal,
folded, tagged, and alias values retain the schema validator's generic error
instead of receiving a more specific diagnostic with a fabricated message or
range. This is the safe branch of the parser-agreement spike's C1 ruling; a
general block/tag/alias projection layer remains out of scope.

**Code action.** In `providers/code_actions.rs`, a `quickfix` titled
`Rewrite with + concatenation` for every diagnostic carrying this code whose
lint produced a suggestion. The diagnostic's typed `data` payload carries a
versioned action discriminator, the analyzed document version, the authored
replacement range, and the already YAML-style-encoded replacement; the
code-action provider never parses the human message and declines stale data
rather than applying it to a newer document snapshot. The
edit replaces the offending `{{ … }}` span's inner text with the suggestion.
For literal blocks this is an authored-range
replacement and therefore preserves the `|-` indicator and all bytes outside
the expression. For single- and double-quoted flow scalars the replacement is
encoded as a fragment of that YAML style before insertion; a bare expression
string must never be inserted into a quoted YAML scalar unchanged. Folded and
tagged scalars receive no action.

**Hover.** No change. The existing hover on a string literal already shows
its value; that value will visibly contain the braces, which is a hint on
its own.

**Implementation-start check.** Confirm that the expression lexer tolerates
raw YAML indentation embedded in expression text. This is expected to hold —
the incident's own ternary parses from decoded text that already contains
newlines and four-space indentation — and is recorded as a check to run, not
as an unknown.

**Architectural note, explicitly out of scope.** The static lifecycle-key
list is a stopgap. The direction it is standing in for is a `SimplifiedSchema`
with schema triggers — a mechanism that brings variables into and out of
scope as the schema dictates, and gives Claudine a supported way to extend
the expression grammar it inherits from the Darkmatter baseline. Under that
mechanism the lifecycle globals would be declared by Claudine's own schema
extension and DMLS would carry no key list at all. Nothing in this fix
builds toward it beyond not making it harder; it is recorded so the debt has
a stated exit and a future reader knows the list was never meant to be
permanent.

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
  is corrected to match its deletion in Resolved Implementation
  Determination 1.
- `darkmatter/docs/inline/interpolation.md`: a "Braces inside string
  literals" rule next to the `{{{ … }}}` recognition rules, stating which
  surfaces rescan and which do not, that a quoted literal is never
  re-scanned within a single pass, and showing the `+` form.
- `darkmatter/docs/lsp/features.md` section 4.3: a row for the new
  diagnostic and quick fix, including the folded-scalar limitation; the
  updated severities for the two existing expression diagnostics; and
  **D7's ladder written down as policy**, where the author of the next
  diagnostic will find it rather than inferring a severity from a
  neighbor. Section 4.1: a note that frontmatter string scalars are now
  scanned for interpolation spans, and that the frontmatter model now
  descends into list items (D6).
- `darkmatter/docs/inline/interpolation.md` also gains: a sentence on why a
  rewrite suggestion is withheld for a span whose declared type is an array
  (objects are already safe), pointing at the array-rendering fix as the
  change that removes the restriction; and **the backslash escape rule (D8)
  alongside the `{{{ … }}}` rules**, giving both spellings, stating that compose preserves
  the backslash and the Markdown renderer resolves it, and showing the case
  it exists for — prose that quotes another template language.
- `.claude/skills/claudine/` and `.claude/skills/darkmatter/` lifecycle and
  expression authoring guidance: the rule and the rewrite, one paragraph
  each. Hash updates via `md hash`.
- `claudine/README.md` and `darkmatter/README.md` only if they enumerate
  validation errors or diagnostic codes today.

### D6. The frontmatter overlay descends into sequences

`lower_mapping` (`dmls/src/overlay/frontmatter.rs`) recurses into
`Node::Mapping` only, so a sequence value yields one entry and the walk
stops. Nothing authored inside a `- item` line has an `FmEntry`. Across
`prompts/`, roughly 68 expression-bearing lines are sequence-nested against
142 mapping-nested: about a third of the real authoring surface is dark to
the editor, and it is the third where lifecycle stacks, action operands, and
`proxy … with` values live.

**Design: item-entries.** `lower_mapping` descends into `Node::Sequence`
and emits **one entry per sequence item**, keyed by its decimal index, in
addition to the entries for a mapping item's keys. The cheaper alternative —
descending but emitting no entry for the item itself — was considered and
rejected: it reaches expressions inside `- item` lines just as well, but it
cannot range a schema problem at the failing array element, which is a
user-visible improvement this fix should not leave behind.

An item is not a mapping key, so it must not be forced through `FmEntry`'s
current “authored key/value entry” fiction. Extend the arena model explicitly:

```rust
pub enum FmEntryRole {
    MappingProperty,
    SequenceItem { index: usize },
}

pub struct FmEntry {
    // existing identity/value fields
    pub role: FmEntryRole,
    pub key_span: Option<SourceSpan>,
}
```

`key` remains the decoded path segment for compatibility; on an item it is the
decimal index. `key_span` is `Some` only for an authored mapping key. Callers
that render or range key-specific behavior operate only on
`MappingProperty`; value diagnostics and `entry_at_offset` use `value_span`
for an item. This avoids inventing a zero-width “key,” underlining the item
value as though it were a key, or firing key completion/navigation on a
synthetic index. Existing mapping entries preserve byte-identical fields and
behavior.

Two changes:

- **`lower_mapping` descends into `Node::Sequence`**, emitting an entry per
  item and per key within it, with correct pointers, spans, parents, and
  depth. The lowering helper may be renamed now that it walks both collection
  kinds.
- **`nested_shape` gains an array-element step**, so a schema path that
  crosses a sequence — `initialize.stack[0].when` — consumes the decimal
  segment against an `is_array` atom and continues with that atom's item type
  rather than looking for a property literally named `0`. A numeric segment
  under a non-array, or a nonnumeric segment where an array item is required,
  fails closed. Union selection applies before the array step and keeps the
  existing merged-arm fallback when no single arm is selected.

**Cost is settled and the arena needs no index.** From
[`spikes/entry-arena-cost.md`](spikes/entry-arena-cost.md), measured over
the 2,441 frontmattered documents in the worktree:

- Entry counts go p90 28 → 104 and max 96 → 2,321 under item-entries
  (keys-only would have been 38 / 1,795). Real prompts are mild: all of
  `prompts/` goes 381 → 662 entries, and the heaviest authoring document,
  `prompts/_implement/implement-plan.md`, goes 32 → 70 with max depth 1 → 5.
  The 2,000-entry outliers are research documents under
  `claudine/docs/research/`, which nobody authors expressions in but which
  are openable like any other file.
- **No pointer-to-index map is needed**, and adding one would be premature.
  Every production lookup stays in the microseconds even at 2,321 entries.
  Pointer round-trip was verified clean, including index-bearing pointers
  and keys containing a literal `/`; no pointer collision is introduced,
  because a YAML node is either a mapping or a sequence and never both.

**Three obligations come with the descent.** The first is performance and
the other two are behavioral:

- **A per-pass memo for `nested_shape_for_completion` is mandatory, not an
  optimization.** It clones the whole schema shape once per ancestor level
  per scalar entry, and descent takes clone counts per diagnostics pass from
  p90 1 / max 63 to **p90 83 / max 3,683**. `diagnostics/scheduler.rs`
  dispatches synchronously and the configured debounce is not yet applied,
  so this runs per `didChange` on the loop thread. Memoize per ancestor path
  for the duration of one pass and discard it after; on real documents that
  is fewer than ten retained shapes. This is the only performance work the
  descent warrants, and it ships in the same change.
- **The quadratic shape must not be introduced.** No production path today
  runs a linear scan inside a per-entry loop: the entry-iterating passes use
  `key_path_at` (O(depth)), never `key_path` (O(n) via `index_of`). That
  shape exists only in a unit test, where it measures 1.5 ms at n=2,321.
  Nothing in this fix may call `key_path(entry)`, `entry_by_pointer`, or
  `entry_by_dotted` inside a loop over `entries()`, and `index_of` gains a
  comment saying so.
- **`test_entry_or_ancestor_falls_back_for_array_index` must be re-cut.**
  It asserts that `/tags/1` falls back to `/tags` with `kind == Sequence`.
  Under item-entries `/tags/1` is an exact hit with `kind == Scalar`, so the
  test fails. It passes unchanged under keys-only, which makes it the
  de-facto switch between the two designs. Rewriting it is a **deliberate
  behavior change** recorded as such in the verification record, not an
  incidental break to be adjusted until green.

**Array schema problems now range the failing item.** `ValidationProblem.path`
is already an RFC 6901 pointer carrying decimal array indices, so once items
have entries, a problem inside an array stops ranging the whole sequence and
starts ranging the element that failed. This is a user-visible improvement
that falls out of the design rather than being built, and it carries its own
acceptance criterion so it is verified rather than assumed.

**This is a structural change to the entry arena, not an additive one.**
Hover, completion, navigation, and every existing frontmatter diagnostic
iterate that arena. All of them will begin producing output at positions
where they previously produced none. That is the intent of the change, but
each is a new behavior surface and none of them was written with list items
in mind:

- **No regression on what already worked.** Every existing diagnostic
  produces the same output on mapping-nested surfaces as it does today.
  This is the primary safety property and is asserted directly, not
  inferred from a green suite.
- **Each capability is decided explicitly.** Hover, completion, and
  navigation work at real keys and scalar values inside list items and are
  suppressed only on the synthetic item marker/index. Positive value-position
  and negative marker-position tests pin the boundary; silence by accident is
  not an acceptable third outcome.
- **The incidental widening is tested, not just the new rule.**
  `EXPRESSION_MALFORMED` and `EXPRESSION_UNKNOWN_IDENTIFIER` will start
  firing inside list items. Combined with D7's severity moves, that is a
  materially different editing experience in a lifecycle-heavy document,
  and it is the change most likely to surprise an author.

### D7. A severity ladder, applied consistently

DMLS has no written severity policy today, and its expression diagnostics do
not follow one: `EXPRESSION_MALFORMED` is `WARNING` and
`EXPRESSION_UNKNOWN_IDENTIFIER` is `INFORMATION`, in both producers. The
policy this fix establishes is one sentence:

> A **warning** means the construct **might** be wrong. An **error** means it
> **will never work**.

Applied to the whole expression family — which, verified against
`dmls/src/diagnostics/codes.rs`, is exactly three codes once this fix adds
its own:

| Code | Today | Under the ladder | Why |
| --- | --- | --- | --- |
| `dm.expression.malformed`, schema-typed frontmatter value with exact scalar projection | `WARNING` | **`ERROR`** | The schema declares the value *is* an expression. A parse failure will never evaluate. Styles without exact projection retain the schema diagnostic instead. |
| `dm.expression.malformed`, document-body span | `WARNING` | **`WARNING`** | A body `{{ … }}` is *inferred* to be an expression. The inference might be wrong. |
| `dm.expression.unknown_identifier` | `INFORMATION` | **`WARNING`** | The identifier might be supplied at runtime by a late-binding global, so it might be wrong rather than certainly wrong. |
| `dm.expression.nested_span_in_literal` | — | **`ERROR`** | On a non-rescanning surface the construct can never work, and compose refuses it. |

No other code in `codes.rs` belongs to the expression family; the schema,
link, wiki, directive, transclusion, style, and security families are not
audited here and keep their severities.

**The frontmatter/body split applies the policy; it does not except it.**
The policy keys on certainty, and the two surfaces differ in exactly that.
For a schema-typed value whose expression text and source range can be
projected exactly, the document has declared what the value is, so a parse
failure is a certainty. Other scalar styles remain schema errors without a
more specific expression code. For a body span the classification is an
inference from two braces, and that inference is frequently wrong in this
very repository: **36 files contain foreign or historical brace syntax in
their bodies.** The bulk of it is Darkmatter's and Claudine's own
documentation describing the deprecated single-pipe fallback —
`{{ name | "default" }}` and `{{env.VAR | "default"}}` across
`2026-04-24-consistent-use-of-logic-operators/` and
`claudine/docs/topics/unified-events.md` — alongside placeholder notation
such as `{{|NAME|}}` in
`renderable/features/_completed/2026-05-26-inline-span/` and `{{#}}` in
`research/docs/commands/list.md`. Beyond the repository the same applies to
any document quoting Handlebars, Liquid, Jinja, or Mustache. Marking that
prose as a hard error would be the tool asserting a certainty it does not
have. D8 gives such documents a way to opt out explicitly.

The new nested-span diagnostic is unaffected by the split: it fires only on
non-rescanning surfaces, and all of those are frontmatter.

**Changing an existing severity is user-visible behavior**, and both moves
need their own acceptance criteria and tests.

**Preconditions on the `ERROR` raise.** From
[`spikes/parser-agreement.md`](spikes/parser-agreement.md), which found the
underlying risk bounded — DMLS has no parser of its own, the two dialects
accept identical strings, and Claudine extends the namespace rather than the
grammar — but attached conditions that are now requirements:

- **Widen the decode guard beyond block scalars.** D3's guard as originally
  written is necessary but not sufficient. `decode_scalar` mis-decoded
  literal/folded blocks and tagged scalars when handed their complete raw
  spelling; aliases are not scalar entries at all. This specification chooses
  the safe branch: the dedicated malformed-expression producer handles only
  untagged plain, single-quoted, and double-quoted scalars with an exact
  `DecodedScalar` map. Every other style keeps the schema validator's generic
  error until an authoritative projection exists.
- **The damage is a wrong message and range on a *true* positive, not a
  false positive.** This corrects what D3 originally claimed. The
  `union_rejected` guard already prevents a spurious report on a valid
  value. What actually breaks is that `when: |-` holding `1 +` reports
  ``Unexpected '|'. Use '||' for logical OR.`` pointed at the block
  indicator rather than at the `1 +`. Cosmetic at `WARNING`; actively
  misleading at `ERROR`, which is why the style gate is a **precondition** for
  the raise rather than a tidy-up beside it. Regression tests assert that the
  misleading dedicated diagnostic is absent and that the schema diagnostic
  remains; exact message/range tests stay on the three safely projected
  styles.
- **Preserve the `union_rejected` guard explicitly, and fix its comment.**
  It is the single reason the frontmatter producer cannot outrank
  Darkmatter's own validation. It reads as a union-arm special case; the
  real invariant is "never report malformed for a value the schema
  validation accepted". The comment is corrected to say so and the
  invariant gains a test, because the descent adds new callers into this
  producer.

**D6 and D7 compound.** D6 widens where these diagnostics fire and D7 makes
them louder. A lifecycle-heavy document that shows nothing today can show
several errors after this fix. That is the intended outcome — the defect
this spec exists for ran an agent to completion in silence — but the
combined effect is larger than either change alone and should be stated in
the PR description as such.

### D8. A backslash escapes an interpolation span

A document that discusses another template language has no good way to stop
Darkmatter scanning its examples. `{{{ … }}}` exists, but it is a literal
*emission* form, not an opt-out: it changes what the author writes and reads
as Darkmatter syntax in prose that is about something else. That is why 36
files in this repository carry brace syntax the composer will try to parse,
and it is the other half of D7's answer to the body-span question.

**Mechanism.** The span scanner declines to treat `{{` as an expression
start when it is preceded by an **odd-length run** of consecutive
backslashes, and **preserves every backslash in its output**. An even-length
run leaves the opener active: in `\\{{ x }}` the first backslash escapes the
second under Markdown rules, so neither one escapes the brace. Counting parity
avoids turning a literal backslash before a real expression into an accidental
opt-out. Compose does not resolve the escape. The
downstream Markdown renderer does, under CommonMark's rule that a backslash
before ASCII punctuation is a literal escape, so `\{{` renders as `{{`.

**Spelling: both `\{{` and `\{\{` work.** `\{{` is the only spelling that
needs an explicit scanner rule. `\{\{` contains no contiguous `{{` opener and
is already ignored by the scanner; its regression test prevents a future
normalization pass from joining the braces. `\{{` is what an author will
naturally type, while `\{\{` escapes both braces explicitly under CommonMark.

Parity is defined on the text passed to `ExpressionFinder`, after any owning
format has decoded it. YAML syntax still applies: inside a double-quoted YAML
scalar an author must escape the backslash for YAML so the decoded value handed
to the scanner contains the intended single backslash. DMLS tests cover both
authored YAML spellings and compare their decision with compose's decoded
input, rather than counting raw source backslashes and disagreeing with the
runtime.

#### Reconciliation with the backslash-preservation fix

`darkmatter/fixes/2026-08-27-preserve-backslash-escapes/spec.md` argues that
compose is a Markdown→Markdown transform and must **not** consume backslash
escapes in body text. This design is consistent with that position rather
than in tension with it, and the consistency is the point: the scanner
*reads* the backslash as a signal and leaves it in place, exactly as that
fix requires. Nothing is consumed, and the rendering of `\{{` is the
Markdown renderer's business, not compose's.

That fix is **merged** — commit `bdf471489`, "fix(darkmatter): preserve
author-written backslash escapes through cleanup" — so there is no
sequencing dependency to manage. Its spec still reads
`status: implemented — awaiting review`, which is status drift rather than
unmerged work. The interaction is nonetheless **verified against that fix's
behavior rather than assumed**: a document containing `\{{ x }}` must
round-trip through compose with the backslash intact, which is that fix's
invariant and this one's mechanism at the same time.

## Scope

In scope:

**Strand 1 — the nested-span rule (D1–D5).**

- D1 through D5 above.
- The DMLS frontmatter-string scan plus `ScalarStyle` and tag-presence capture
  on `FmEntry` (D3), because without them the editor half of the outcome is
  unreachable or can be ranged against the wrong authored bytes.
- The scalar-style gate around `decode_scalar` in
  `dmls/src/providers/frontmatter.rs`, including exact projection for the
  three supported styles and generic-schema fallback for the rest (D3/D7).
- Correcting both live instances. This is an outstanding task, not a
  completed one:
    - `prompts/_reviews/review-spec-inline.md` was partially edited by hand
      and is currently worse than before. `success.say` reads
      `… + ctx.area + "" has completed"`, a stray `"` that makes the
      expression malformed, and `failure.say` still contains
      `{{ctx.area}}` inside a literal. Acceptance Criterion 3 fails today.
    - `prompts/commit.md`'s `resides_in`. Ken is separately moving that
      file to `as_csv(…)` under the array-rendering fix; the two edits are
      independent and neither blocks the other.
- Two regression fixtures, both captured from `HEAD`, not from the working
  tree: the incident document
  (`git show HEAD:prompts/_reviews/review-spec-inline.md`, where both `say`
  values carry the defect — the working tree's `failure.say` still carries
  it too, but its `success.say` is a malformed half-edit), and
  `git show HEAD:prompts/commit.md`'s `resides_in` value with its single
  quotes, `\n` escape, two nested spans in one literal, and array-valued
  span. The working tree of `commit.md` already holds a hand-written `+`
  rewrite and is not the fixture.

**Strand 2 — full frontmatter coverage (D6).** Sequence descent in the
frontmatter lowering walk, explicit `SequenceItem` arena roles, the
array-element step in `nested_shape`, `when` / `until` / `while` declared
expression-typed in the schema, and the regression and per-capability work the
descent obliges.

**Strand 3 — the severity ladder (D7).** The written policy, the new
diagnostic at `ERROR`, `EXPRESSION_MALFORMED` raised to `ERROR` on
schema-typed frontmatter values with exact scalar projection and left at
`WARNING` on body spans,
`EXPRESSION_UNKNOWN_IDENTIFIER` raised to `WARNING`, and the three
preconditions the parser-agreement spike attached to the raise — the safe
scalar-style gate, projection/fallback regression tests, and the preserved
`union_rejected` invariant with a corrected comment.

**Strand 4 — the backslash escape (D8).** Odd/even backslash-run handling for
`\{{`, the already-inert `\{\{` spelling pinned by regression, every
backslash preserved in compose output, and the round-trip verified against the
merged backslash-preservation fix.

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
- Unifying Darkmatter's two array renderings. That is
  `darkmatter/fixes/2026-09-13-unify-array-rendering/spec.md`, which also
  lifts D1's suggestion suppression. Nothing here depends on it landing.
- An `md lint` or `md validate` CLI surface for expressions (ruled; see
  Resolved Questions).
- A schema-driven replacement for the static lifecycle-key list (the
  architectural note at the end of D3).
- Severity review of the schema, link, wiki, directive, transclusion, style,
  and security diagnostic families. D7's ladder is stated as general policy
  but applied here only to the expression family.

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
- **Suppression is type-directed.** A span whose declared type is an array
  (`ctx.dirty_package_areas`) is diagnosed with `suggestion: None`; a declared
  object, declared scalar (`ctx.repo_name`), and untyped span are each offered
  a suggestion. The array test names the array-rendering fix as the change
  that will flip its expectation, so whoever re-cuts it knows it was
  deliberate rather than a gap. Tests also prove the untyped lint entry and
  the `_with_types` entry agree for `Unknown`.

### Darkmatter L1 — backslash escape (D8)

- `\{{ x }}` and `\{\{ x }}` in a document body are not treated as
  expression spans, produce no diagnostic of any expression code, and
  survive compose with the backslash intact.
- `{{ x }}` with no backslash still interpolates, so the escape did not
  disable the feature.
- `\\{{ x }}` is still an active expression opener, while `\\\{{ x }}` is
  escaped; all backslashes survive compose. These cases pin odd/even parity.
- A backslash elsewhere in body text is untouched, which is the merged
  backslash-preservation fix's invariant and is asserted here rather than
  assumed.
- A document containing the deprecated single-pipe examples from
  `claudine/docs/topics/unified-events.md`, escaped, produces zero
  expression diagnostics.
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
  synthesized literals; the last is a rescanning surface. It also does not
  reject the same whole-value expression under a non-lifecycle frontmatter
  key, because that surface is not in Claudine's single-pass inventory.
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
  path. Explicit-relative, implicit-relative, `@`, `&`, and `^` fixtures prove
  it uses the same captured `FileResolutionContext` and source-relative
  derivation as execution on every OS. Where the reference is itself
  interpolated, the run starts and the defect is caught at that step's turn,
  still before that step's provider is spawned.
- D4: a frontmatter value holding raw template text reaches the runtime
  guard, and the stderr names the lifecycle key and carries the new reason
  and hint. The old hint text must not appear.

### DMLS L1 (`darkmatter/dmls`)

- Opening the incident file produces two
  `dm.expression.nested_span_in_literal` diagnostics at `ERROR`, one per
  `say`, each ranged on the inner `{{ … }}` inside the block scalar, with
  the source `darkmatter.frontmatter`.
- A body interpolation with the same defect produces **no** diagnostic of
  this code, and the document still composes.
- The code action rewrites the block scalar's expression text and leaves
  the `|-` indicator and indentation untouched; the resulting document
  produces zero diagnostics of this code.
- Single- and double-quoted flow scalars project the nested-span range through
  `DecodedScalar`; their code actions YAML-escape the replacement fragment,
  preserve the surrounding scalar quotes, and leave a parseable document.
- The code action reads only its typed diagnostic payload, is absent when the
  lint has no suggestion, and is declined after the document version changes.
- A folded (`>`) scalar carrying the defect is ranged on the whole scalar
  and offers no quick fix.
- An `Expression`-typed schema property authored as a `|-` block no longer
  produces a spurious `EXPRESSION_MALFORMED` report. This test must pass
  before `EXPRESSION_MALFORMED` is raised to `ERROR`.
- `err`, `timing`, and `current` produce no unknown-identifier report under
  a lifecycle key, and **do** produce one outside lifecycle keys. Both
  directions are asserted; the second is what makes the scoping meaningful
  rather than a blanket allowlist.
- The `no_side_effects.rs` and `packaging_contract.rs` suites stay green.

### DMLS L1 — sequence descent (D6)

- A scalar inside a `- item` line gets an `FmEntry` with
  `role == SequenceItem`, `key_span == None`, and the correct decimal path
  segment, pointer, key path, value span, parent, and depth. Mapping entries
  retain `role == MappingProperty` and their exact key spans.
- `nested_shape` resolves a path that crosses a sequence, such as
  `initialize.stack[0].when`.
- A `when` predicate inside a stack item, declared expression-typed in the
  schema, is reached by `expression_values`, parsed with the condition
  dialect, and produces the nested-span diagnostic when it carries the
  defect.
- A nested span in a stack action operand and in a `proxy … with` value is
  diagnosed, which is the coverage D2 previously held alone.
- **No regression on mapping-nested surfaces.** Every existing diagnostic
  produces byte-identical output on a corpus of mapping-only documents
  before and after the descent. Asserted directly.
- Hover, completion, and navigation each have a positive test at a real key
  or scalar value inside a list item and a negative test at the synthetic
  item marker/index. No capability is left untested there.
- `test_entry_or_ancestor_falls_back_for_array_index` is re-cut for
  item-entries: `/tags/1` is an exact hit with `kind == Scalar`. The
  verification record names it as the deliberate design switch it is.
- A schema problem inside an array ranges the failing element rather than
  the whole sequence.
- The `nested_shape_for_completion` memo holds: a diagnostics pass over
  `prompts/_implement/implement-plan.md` performs at most one shape clone
  per distinct ancestor path, not one per ancestor level per scalar entry.
- No production path calls `key_path(entry)`, `entry_by_pointer`, or
  `entry_by_dotted` inside a loop over `entries()`. Asserted by review and
  recorded, since it is a shape rather than a value.

### DMLS L1 — severity ladder (D7)

- `dm.expression.malformed` is reported at `ERROR` on a safely projected,
  schema-typed frontmatter value and at `WARNING` on a document-body span;
  `dm.expression.unknown_identifier` is `WARNING` in both producers.
- Untagged plain, single-quoted, and double-quoted scalars holding `1 +`
  produce the dedicated malformed-expression `ERROR` with an exact projected
  range and parser message. Literal/folded blocks (`|`, `|-`, `|+`, `>`, `>-`,
  `|2-`), tagged scalars, and aliases produce no
  `EXPRESSION_MALFORMED`; the schema diagnostic remains, proving the style
  gate removes a misleading diagnostic rather than hiding invalid input.
- The `union_rejected` invariant has a test: no malformed report is emitted
  for a value the schema validation accepted, including
  `when: '{{ ctx.area }}'` and `when: "$(git branch) == 'main'"`.
- The same two pending values are deferred explicitly by Darkmatter's exported
  `is_pending_expression_value` authority in both schema validation and DMLS;
  the test would still pass if the `union_rejected` guard were refactored.
- A body containing Handlebars, Liquid, or Jinja syntax produces at most
  `WARNING`, never `ERROR`.
- Existing suites that assert on these severities are re-cut, and the
  verification record names them as intentional assertion changes rather
  than regressions.
- The existing `EXPRESSION_MALFORMED` code action still offers itself at
  the new severity.

### Evidence

macOS L1 for all three areas locally through `just test`. Linux, native
Windows, and WSL2 through CI on the PR. The DMLS L2 editor suite is not
required for this change. The verification record lists the failing-before
proof for each new test group.

## Acceptance Criteria

**Strand 1 — the nested-span rule.**

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
   recognizer; `is_whole_value_span` is the shared syntactic classifier; the
   Claudine lifecycle inventory and DMLS stopgap list each pass their parity
   test against `darkmatter/docs/schemas/claudine.yaml`; and both Darkmatter
   property tests pass.
6. DMLS emits `dm.expression.nested_span_in_literal` for whole-value
   lifecycle scalars with the style-specific ranges and source in D3, emits
   nothing for body spans, mixed frontmatter strings, or arbitrary
   non-lifecycle whole-value keys, and each offered quick fix produces a
   parseable document with zero such diagnostics.
7. The event-time guard's message names the lifecycle key and the new
   reason, selected from a typed reason rather than by message matching;
   the "resolve the missing path or variable" hint no longer appears for a
   surviving span.
8. A span whose declared type is an array is diagnosed with no rewrite
   suggestion; a declared object, declared scalar, and untyped span are all
   offered one. Nothing in this fix depends on the array-rendering fix having
   landed.

**Strand 2 — full frontmatter coverage.**

9. Scalars inside list items carry explicit `SequenceItem` `FmEntry` nodes
   without fabricated key spans, `nested_shape` resolves a path crossing a
   sequence, and the nested-span rule reaches `when` /
   `until` / `while` predicates, stack action operands, and `proxy … with`
   values in the editor.
10. Every existing diagnostic produces identical output on mapping-nested
    surfaces before and after the descent; hover, completion, and navigation
    work at real keys/values inside list items and are suppressed on the
    synthetic item marker/index, with both directions tested.
11. A schema problem inside an array ranges the failing element rather than
    the whole sequence.
12. The `nested_shape_for_completion` memo is in place, and no production
    path runs a linear arena scan inside a loop over `entries()`.

**Strand 3 — the severity ladder.**

13. `dm.expression.malformed` reports at `ERROR` on safely projected,
    schema-typed frontmatter values and `WARNING` on body spans,
    `dm.expression.unknown_identifier` at `WARNING`, and
    `dm.expression.nested_span_in_literal` at `ERROR`; the ladder is written
    down where a future diagnostic author will find it.
14. The style gate limits the dedicated malformed-expression producer to
    untagged plain, single-quoted, and double-quoted scalars with exact
    projection. Literal/folded blocks, tagged scalars, and aliases retain the
    schema diagnostic and never receive a fabricated expression message or
    range. This lands before or with the `EXPRESSION_MALFORMED` raise.
15. The `union_rejected` invariant is preserved, its comment states the real
    rule, and a test pins it. Pending-value deferral is also explicit through
    one exported Darkmatter predicate used by validation and DMLS rather than
    depending on that guard incidentally.

**Strand 4 — the backslash escape.**

16. `\{{` and `\{\{` both opt a span out of scanning, every backslash
    survives compose unchanged, odd/even backslash runs are distinguished,
    and unescaped `{{ … }}` still interpolates.

**All strands.**

17. `err`, `timing`, and `current` resolve under lifecycle keys and are
    reported as unknown elsewhere.
18. Docs listed in D5 are updated in the same change, and skill hashes are
    refreshed with `md hash`.
19. L1 green on macOS locally and on Linux, Windows, and WSL2 in CI.

## Resolved Questions

Every question this fix opened has been ruled. They are recorded with their
reasoning so a future reader does not reopen them without the context.

1. **Should `md validate` grow an expression-lint pass?** **Ruled: no.** No
   `--expressions` flag on `md validate` and no new `md lint`. Expression
   linting stays with DMLS, which covers authoring, and Claudine, which
   rejects at prepare time. Revisit when a second lint rule exists and a
   batch surface has a reason to exist.
2. **Severity for the new diagnostic: warning or error?** **Ruled: `ERROR`,
   under a general policy** — warnings mean "might be wrong", errors mean
   "will never work" (D7). The consistency argument originally offered for
   `WARNING` rested on a false premise: DMLS does not render every
   expression problem as `WARNING`. `EXPRESSION_MALFORMED` was `WARNING`
   while `EXPRESSION_UNKNOWN_IDENTIFIER` was `INFORMATION`, in both
   producers, so the ladder was already uneven rather than flat. D7 makes it
   deliberate and moves both existing codes.
3. **Should `err` / `timing` / `current` be known to DMLS everywhere, or
   only under lifecycle keys?** **Ruled: only under lifecycle keys**, since
   that is the only place they exist. This reverses the earlier
   recommendation in this document, which favored recognizing them
   everywhere to avoid teaching DMLS about Claudine's schema. The coupling
   is accepted knowingly as a stopgap, with its drift cost stated in D3 and
   its intended replacement recorded in D3's architectural note.
4. **Does the editor need to see `when` / `until` / `while` predicates?**
   **Ruled: yes, and the blocker is removed rather than accepted.** The
   predicates are declared expression-typed in the schema and reached
   through the existing schema-driven path; D6's sequence descent and
   array-element step are what make them reachable at all. Two verified
   facts support this: `expression_diagnostics` already parses with the
   condition dialect, so no parser selection is needed; and the original
   blocker — `lower_mapping` recursing into `Node::Mapping` only, leaving
   nothing inside a `- item` line with an `FmEntry` — is the thing D6 fixes.

5. **Should DMLS assert a hard error on foreign template syntax in a
   document body?** **Ruled: no, twice over.** The parser-agreement spike
   raised this as the largest practical risk of the severity raise, having
   measured that `{{#each items}}`, `{{ x | default: 1 }}`, `{{ a|upper }}`,
   `{{> partial}}` and `{{ $json.id }}` all produce malformed reports in any
   `.md` file in the workspace, with or without frontmatter. Two decisions
   answer it together: D7 leaves body spans at `WARNING`, because a body
   brace is an inference rather than a declaration; and D8 gives a document
   an explicit way to opt out. Neither a per-code severity override in
   `DmlsConfig` nor restricting the body producer to recognizably Darkmatter
   documents is needed.
6. **Is every whole-value frontmatter scalar a single-pass surface?**
   **Ruled: no.** `is_whole_value_span` selects the typed one-call branch, but
   the frontmatter pipeline can invoke interpolation again after shell
   expansion. The hard nested-span diagnostic therefore requires both the
   whole-value shape and membership in Claudine's lifecycle single-pass
   inventory. Arbitrary frontmatter keys are inventoried for existing
   diagnostics but do not receive this `ERROR`.
7. **Should DMLS build source maps for block, tagged, and alias values merely
   to support the severity raise?** **Ruled: no.** The dedicated malformed-
   expression diagnostic is gated to the three scalar styles with an exact
   existing `DecodedScalar` projection. Other styles retain the schema
   validator's generic error. The new nested-span rule still gives literal
   blocks precise ranges by parsing each raw outer span; folded and tagged
   values use a whole-scalar range, and aliases have no expression diagnostic
   at the alias site.

No questions remain open. The implementation determinations below were gaps
in the earlier draft and are now part of the design.

## Resolved Implementation Determinations

1. **The disposition of `validate_no_interpolation_leaks`.** It exists in
   `claudine/lib/src/composition/lifecycle/validate.rs`, walks
   `iter_stack_expression_surfaces` with `visit_string_literals` hunting for
   surviving spans in literals, is exported from `lifecycle/mod.rs`, and has
   tests in `lifecycle/tests/diagnostics.rs` and
   `lifecycle/tests/validation.rs`. No production call site was found in
   `composition/prepare/` or in `claudine/cli/src/`. The new validator
   overlaps it heavily.

   **Decision: fold and delete.** Repository call-site inspection confirms the
   function has no production caller. Its authored-source coverage moves to
   `validate_no_nested_spans_in_literals`; its surviving-runtime-value coverage
   remains with `reject_surviving_spans`, where dynamic template text can be
   classified honestly at event time. The old function and tests that exercise
   only the dead entry point are deleted; useful cases are recut through the
   new validator or runtime guard. Wiring the old tree-only scan would preserve
   the authored-vs-synthesized ambiguity that caused this defect. In all cases,
   `claudine/docs/topics/lifecycle.md`'s claim that it "still runs" for
   non-lifecycle surfaces is corrected in the same change.
2. **The raw-source route for stack surfaces.** D2 requires raw YAML text
   for surfaces that `iter_stack_expression_surfaces` reports as parsed
   trees. **Decision: extend the canonical iterator.** Build one
   `LifecycleSourceMap` from raw frontmatter, keyed by the iterator's canonical
   property path, and attach the authored scalar/span to each
   `LifecycleExpressionSurface`. A configured parsed surface with no matching
   source record is an internal validation error, not a silent skip. There is
   no second surface walker; every existing validator continues to consume the
   same iterator, and only validators needing source text read the added field.
3. **The dotted-path array spelling.** `entry_by_dotted` matches by exact
   string, and the descent must produce a spelling that agrees with
   Darkmatter's own dotted-path producers. If the overlay emits
   `initialize.stack[0].when` and a producer emits
   `initialize.stack.0.when`, the lookup misses and the caller `continue`s —
   a silently dropped diagnostic. `style_diagnostics`
   (`diagnostics/frontmatter.rs`) is the live instance of that pattern.
   **Decision: bracketed indices** (`initialize.stack[0].when`). A shared
   formatter consumes typed path segments (`Key(&str)` versus `Index(usize)`),
   so a mapping key literally named `0` remains `.0` while a sequence item is
   `[0]`. Lowering and every producer that emits an address use that formatter;
   a round-trip test covers numeric mapping keys and nested arrays. This is a
   spelling decision, not a structural one, but getting it wrong fails
   silently.
4. **The pending-value deferral.** Darkmatter's `expression` format
   validator accepts any value containing `$(` or `{{` (`format.rs`),
   deferring on values that are not yet resolved. DMLS has no equivalent,
   and the `union_rejected` guard hides the difference **coincidentally**
   rather than by design. The descent adds new callers into that producer.
   **Decision: single-source and apply it explicitly.** Export Darkmatter's
   passive `is_pending_expression_value` predicate from the schema-format
   layer and call it from both validation and DMLS before parsing. Preserve the
   `union_rejected` guard as a second invariant, not as the accidental source
   of deferral semantics. The helper only classifies text; it performs no I/O
   or evaluation.
5. **Per-capability behavior inside list items (D6).** The sequence descent
   makes hover, completion, and navigation fire where they never fired
   before. For each, decide "works correctly" or "suppressed inside sequence
   items", and record which, with the test that holds it. The decision is
   **Decision:** hover, completion, and navigation all operate at real mapping
   keys and scalar values inside sequence items, using the array item schema.
   All three are suppressed on the synthetic item index/sequence marker itself,
   which has no authored key and no independent schema declaration. Each
   direction has a positive value-position test and a negative marker-position
   test; silence by accident is not an acceptable third outcome.

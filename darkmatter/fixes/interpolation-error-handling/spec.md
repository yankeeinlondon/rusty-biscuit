---
created: 2026-06-13
reviewed: true
status: ready for planning and implementation
severity: bug
area: darkmatter
component: markdown/compose/interpolation
---

# Interpolation: Fail Loudly on Invalid Grammar, Support `&&`, Preserve Whole-Value Types

## Problem Statement

A Claudine composition prompt declared a frontmatter flag:

```yaml
file: "{{ctx.repo_root}}/claudine/docs/research/agent-models/{{state.file}}"
update: "{{file_exists(file) && markdown_file_empty(file)}}"
```

After `claudine sequence … --dry-run`, the `update` value came back **unchanged**
— still the literal string `"{{file_exists(file) && markdown_file_empty(file)}}"`.
Every other templated value (`file`, `state.*`, …) resolved. No error was
reported; the document composed "successfully".

This is not a one-off. It exposes three distinct defects in Darkmatter's
interpolation engine, in descending order of severity:

1. **Silent degradation of invalid interpolation (dangerous).** When a `{{ }}`
   expression fails to *parse*, the default compose path (`fail_fast = false`)
   records a non-fatal warning and **leaves the literal `{{ … }}` text in
   place**. Darkmatter has already decided the value is an interpolation target
   — it found a `{{ … }}` span — so a grammar error is unambiguously an
   authoring mistake. Silently emitting the raw template text is wrong: the
   `{{ … }}` braces are Darkmatter syntax that must never appear in output, and
   downstream consumers cannot tell a failed expression from intended content.
   Worse, a leftover literal like `"{{ … }}"` is a **non-empty string**, so a
   `::block when="update"` reads it as **truthy** and `when="!update"` as
   **falsy** — silently inverting the author's intended branch logic.

2. **`&&` is rejected in interpolation mode.** The lexer errors on `&&` unless
   it is in condition mode (`when="…"`). Logical AND is meant to be a
   first-class interpolation operator. The parser's interpolation ladder
   already routes through `parse_logical_and`; only the lexer gates it.

   **Reader note.** This is an intentional standards change relative to the
   completed "Consistent Use of Logic Operators" spec, which preserved `&&` as
   invalid in interpolation mode. The implementation has since grown a shared
   expression parser where `parse_fallback` already delegates through
   `parse_logical_and`; enabling `&&` in interpolation aligns the live grammar
   with the documented precedence ladder and avoids a mode-specific exception
   that has no syntax conflict.

3. **Whole-value expressions lose their type.** A frontmatter value that is
   exactly one `{{ expr }}` is stringified — a boolean result becomes the
   string `"true"`/`"false"` rather than a JSON boolean. Since a non-empty
   `"false"` string is truthy, a precomputed boolean flag is unusable in
   `when=` conditions even once it evaluates. The same typed path should also
   preserve numbers, nulls, arrays, and objects when an expression returns or
   selects those values; frontmatter is structured data, not just scalar text.

The reported expression hit all three: `&&` made it a *grammar* error (defect 1
+ 2), and even with valid grammar the boolean result would have stringified
(defect 3). The function-name typo (`markdown_file_empty` — the registered name
is `markdown_body_empty`) is a separate authoring error and is **not** a
Darkmatter bug; see [Non-Goals](#non-goals-out-of-scope).

## Root Cause

### Defect 1 — parse errors degrade silently (intended, and wrong)

`interpolate_text` (`markdown/compose/interpolation/rewrite.rs`) gates *all*
failures — both parse and evaluation — on `fail_fast`:

```rust
match parse(&loc.expression) {
    Ok(expr) => match evaluator.eval(&expr) {
        EvalResult::Value(replacement) => { /* substitute */ }
        EvalResult::Error { .. } if fail_fast => return Err(…),
        EvalResult::Error { message, original } => { warnings.push(…); /* leave literal */ }
    },
    Err(e) if fail_fast => return Err(…),
    Err(e) => { warnings.push(…); /* leave literal */ }   // <-- defect 1
}
```

The default `ComposeOptions::fail_fast` is `false` (`compose/types.rs:637`), so
in normal use a parse error becomes a warning and the original `{{ … }}` text
survives. This is currently **deliberate** and locked in by tests:

- `compose/mod.rs::test_interpolation_parse_error_preserves_original`
- `compose/mod.rs::test_interpolation_bare_pipe_produces_parse_error`

These tests encode the behavior this fix overturns and will be rewritten (see
[Test Plan](#test-plan)).

The defect is the conflation of two error classes. A **parse/grammar error**
(the lexer/parser cannot build an AST) means the text is not a valid expression
at all — it can never be salvaged, and leaving raw braces in output is never
correct. An **evaluation error** (a syntactically valid expression whose
variable is missing or whose function is unknown) is a different concern with a
legitimate lenient mode (optional variables resolving to empty).

### Defect 2 — lexer rejects `&&` in interpolation mode

`compose/expression/lexer.rs`, `next_token`, the `'&'` arm:

```rust
'&' => {
    self.advance();
    if self.current_char() == Some('&') {
        self.advance();
        match self.mode {
            ParseMode::Condition => Ok(Token::AndAnd),
            ParseMode::Interpolation => Err(LexerError::new("Unexpected character: '&'", start_pos)),
        }
    } else { Err(…) }
}
```

`&` has no other meaning in interpolation mode (unlike `||`, which is overloaded
as the fallback operator), so there is nothing to disambiguate. The parser is
already wired: `parse_ternary_branch` → `parse_fallback` → `parse_logical_and`
consumes `Token::AndAnd` and lowers `a && b` to `and(a, b)` in *both* modes.
The lexer is the sole gate.

### Defect 3 — whole-value expressions stringify

Frontmatter interpolation (`compose/frontmatter_interpolation.rs::rewrite_value`)
always routes `Value::String` through `interpolate_text`, which performs **text
substitution** and returns a `String`. When the entire value is one expression,
the typed result (`EvalValue::Bool`) is flattened to its string rendering
(`scalar_string`). There is a typed entrypoint available — `Evaluator::eval_value`
→ `EvalValue` (`compose/interpolation/evaluator.rs:267`) — that is not used on
this path.

## Goals

1. **A parse/grammar error in any recognized `{{ … }}` is always a hard error**,
   regardless of `fail_fast`. The error must name the offending expression and
   the parse/lexer reason, and (where available) the frontmatter key or source
   location. No composed output is produced when grammar is invalid.
   *(Decision: "grammar errors only".)*

2. **The strict rule applies to all interpolation surfaces** — frontmatter
   values, body text, transclusion/`when` interpolation, and shell-expansion
   interpolation. One rule, everywhere a `{{ … }}` span is recognized.
   *(Decision: "all surfaces".)*

3. **`&&` is a valid interpolation operator**, lowering to `and(a, b)` with the
   same precedence as condition mode (binds tighter than `||`/fallback).

4. **A frontmatter value that is exactly one `{{ expr }}` preserves its
   evaluated JSON type** (string/bool/number/null/array/object), not its string
   rendering. Embedded expressions (an expression with surrounding literal text,
   or one of several in a value) continue to stringify, because the surrounding
   text forces a string result. *(Decision: "preserve type".)*

5. No raw `{{ … }}` braces ever appear in composed output. (Falls out of goals 1
   and the eval-error refinement below.)

## Behavior Specification

### Error taxonomy

| Failure | Class | `fail_fast=false` | `fail_fast=true` |
|---|---|---|---|
| Lexer error (e.g. bare `&`, bad char) | **Grammar** | **Hard error** | Hard error |
| Parser error (e.g. `@invalid`, dangling op) | **Grammar** | **Hard error** | Hard error |
| Unsupported-in-mode operator after this fix (none for `&&`) | **Grammar** | Hard error | Hard error |
| Missing variable | Evaluation | Resolves to empty string in text, null in whole-value frontmatter | Same |
| Unknown function | Evaluation | Warn + replace with empty string in text, null in whole-value frontmatter | Hard error |
| Function runtime error (e.g. type mismatch) | Evaluation | Warn + replace with empty string in text, null in whole-value frontmatter | Hard error |

Grammar errors no longer depend on `fail_fast`. Evaluation errors keep the
existing `fail_fast` semantics.

### Eval-error refinement (replace with empty/null, never leave literal)

Today a non-fail-fast **evaluation** error also leaves the raw `{{ … }}` literal
in place — the same dangerous truthy-string leak as defect 1, just for a
different error class. Under "grammar errors only" we do **not** promote eval
errors to hard failures, but lenient must mean *replace + warn*, **not** *leave
literal braces*. The replacement is surface-specific:

- Text substitution surfaces (body text, embedded frontmatter expressions,
  shell-expansion interpolation, and interpolated directive arguments) replace
  the failed span with the empty string.
- Whole-value frontmatter expressions replace the value with JSON `null`.

This guarantees goal 5 (no raw braces in output) without changing the lenient
contract for optional variables.

### `&&` semantics

- Valid in both interpolation and condition mode; lexes to `Token::AndAnd`.
- Lowered to `and(a, b)`; left-associative; binds tighter than `||`/fallback,
  looser than comparison. `a == b && c == d` → `and(a == b, c == d)`.
- Single `&` remains a lexer error in both modes.

### Whole-value type preservation (frontmatter only)

A frontmatter string value qualifies for type-preserving evaluation when
`ExpressionFinder::find_all_plain(value)` yields exactly one span and that span
covers the entire (untrimmed-of-delimiters) value — i.e. the value is `{{ … }}`
and nothing else. In that case evaluate through a typed path and store the
resulting JSON `Value` (string/bool/number/null/array/object). If the current
`EvalValue` helper remains scalar-only, add a typed evaluation helper that
returns `Result<serde_json::Value, String>` rather than serializing arrays or
objects through `to_string()`. All other strings keep text-substitution
behavior. Type preservation is scoped to frontmatter; body text is inherently
string output.

Replacement counts and warning behavior must still be reported consistently:
a lenient eval error that replaces a span with empty/null counts as one handled
interpolation, so the recursion guard does not keep rescanning the same failed
expression.

## Non-Goals (Out of Scope)

- **The `markdown_file_empty` typo.** The registered function is
  `markdown_body_empty` (alias `markdownbodyempty`). The user's document must be
  corrected separately; no alias will be added. After this fix it surfaces as an
  unknown-function *evaluation* error (warn + empty/null under default, hard
  error under `fail_fast`), not as a silent literal.
- **Adding `||`-as-logical-OR to interpolation mode.** `||` stays the fallback
  operator in interpolation; logical OR remains condition-mode only. Unchanged.
  `&&` in interpolation is intentionally logical AND, so an expression such as
  `a && b || c` parses as fallback over the result of `and(a, b)`.
- **Type preservation in body text.** Body interpolation is string output by
  definition.
- **New expression functions or operators** beyond enabling `&&`.
- **Changing `fail_fast` defaults** or the schema-validation hard-error path.

## Affected Code

- `darkmatter/lib/src/markdown/compose/expression/lexer.rs` — `'&'` arm; doc
  comments on `ParseMode`, `Token::AndAnd`, `with_mode`; interpolation-mode
  tests that currently assert `a && b` errors.
- `darkmatter/lib/src/markdown/compose/expression/parser.rs` — grammar/precedence
  doc comments (`&&` is "both modes"); `parse_condition` doc. The existing
  parser behavior is already structurally compatible with this change.
- `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs` — split parse
  vs eval handling so parse errors always error; eval errors replace the span
  with empty string under lenient mode and emit a warning.
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs` —
  whole-value type-preserving path via a JSON-preserving evaluator; lenient eval
  errors produce JSON null and a warning.
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs` or
  `darkmatter/lib/src/markdown/compose/expression/mod.rs` — expose a typed
  evaluation path that can return the full `serde_json::Value`; do not rely on
  scalar-only `EvalValue` if it would serialize arrays/objects.
- Any caller relying on warn-and-continue for grammar errors (audit
  `compose/mod.rs` interpolation operation wiring).

## Test Plan

Rewrite the tests that lock in the old contract, and add coverage for each goal:

- **Rewrite** `test_interpolation_parse_error_preserves_original` and
  `test_interpolation_bare_pipe_produces_parse_error` → assert a **hard error**
  with `fail_fast=false`.
- **Keep** `test_interpolation_parse_error_fail_fast_returns_error` and the
  bare-pipe fail_fast message test (still valid).
- **Grammar always errors (all surfaces):** body text, frontmatter value,
  transclusion `when`, shell-expansion interpolation each hard-error on a
  malformed `{{ }}` under default options.
- **`&&` lexes + evaluates** in interpolation mode (lexer token test; parser
  `and(a,b)` lowering; end-to-end frontmatter `{{ a && b }}` → boolean).
- **Type preservation:** whole-value `{{ exists && empty }}` → JSON `true`/
  `false`; whole-value `{{ items }}` and `{{ object_value }}` preserve arrays
  and objects; embedded `prefix {{ flag }}` → string.
- **Eval leniency unchanged for variables:** missing variable → empty; and
  (per refinement) unknown function under default → warning + empty/null, no
  literal braces in output. Include both embedded text and whole-value
  frontmatter cases.
- **Regression:** the reported sequence document, with `&&` valid and the
  function name corrected, yields a typed boolean `update` and correct
  `when="update"` / `when="!update"` branching.
- **No-rescan regression:** a lenient eval error is replaced once and does not
  trigger interpolation-depth warnings by leaving the same failed `{{ … }}` in
  place.

## Risks

- **Behavior change for existing documents.** Any document currently relying on
  a malformed `{{ }}` surviving as literal text will now hard-error. This is the
  intended correction; a workspace grep for surviving `{{` in composed outputs
  should be run before rollout.
- **`fail_fast` consumers.** Callers that pass `fail_fast=false` expecting
  warn-and-continue for *grammar* errors lose that behavior. Audit Claudine's
  compose/inline-compose/sequence call sites.
- **Structured-value frontmatter.** Preserving arrays/objects can change the
  type seen by schema validation and downstream callers that previously saw a
  JSON-looking string. This is intended when the full value is one expression,
  but the implementation should add explicit tests around schema validation so
  the new typed value is validated as structured data.
- **Warning visibility.** In lenient mode, replacing an unknown function with
  empty/null prevents branch inversion but can still mask a typo if warnings are
  hidden by the caller. Claudine should surface compose warnings in `compose`,
  `inline-compose`, and `sequence` dry-run output.
- **Recursive interpolation.** Replacing eval errors with null/empty means those
  spans must count as handled for recursion-depth accounting; otherwise a
  failed expression could be rescanned until the depth guard fires.

## Open Questions

None. This review resolves the prior open point by specifying surface-specific
empty/null replacement for lenient evaluation errors, and it treats `&&` in
interpolation as an intentional grammar expansion rather than accidental drift.

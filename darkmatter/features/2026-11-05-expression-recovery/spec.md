---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
kind: feature
created: 2026-11-07
status: draft-spec
area: darkmatter
reviewed: false
clarified: false
implemented: false
review_iterations: 0
packages:
    - darkmatter
    - darkmatter-lsp
related:
    - 2026-11-04-loading-data
---

# Expression Recovery

Add one expression-level construct, such as `try(expr, fallback)`, that
evaluates an expression and, when its evaluation fails with a recoverable
runtime error, evaluates a fallback instead. Today no expression can absorb
an error, so every reader function that can fail has to choose between
failing the document and growing its own `try_` variant.

## Motivation

2026-11-04-loading-data decided against a `try_load_data` and recorded that
"a general recovery construct belongs in its own spec rather than one
function at a time". This is that spec.

## Current behavior

Verified against the code on 2026-11-07:

- **An evaluation error fails the document.** An interpolated expression that
  cannot be parsed or evaluated fails full-document composition whatever `fail_fast`
  says. `load_data` failures (`dm.data.*`) are authoring-fatal and propagate
  through callbacks, interpolation, conditions, and nested composition; they
  also fail the reference check before any command runs.
- **`||` does not recover.** `Expr::LogicalOr` evaluates its left operand
  with `?` (`compose/expression/mod.rs`), so an error on the left propagates
  and the right operand is never reached. `||` falls back only on a falsy
  value. `load_data(f, "string[]") || []` therefore fails composition, and a
  test asserts it.
- **Absence has a pattern; failure has none.** A file that may be missing is
  guarded with `file_exists(f) ? load_data(f, "string[]") : []`. A file that
  exists but is malformed, or does not match `as`, always fails.
- **One function recovers on its own.** `try_frontmatter(file)` returns
  `{ok, value, error}` instead of failing, for Markdown frontmatter only.
  Every other reader in the catalog (`frontmatter`, `load_data`,
  `markdown_title`, `validate_schema`, and the rest marked `fallible` in
  `schemas/partials/functions.yaml`) fails the expression.
- **Arguments are evaluated before a call.** An ordinary catalog function
  receives already-evaluated values; only callback arguments (`map`,
  `filter`) are deferred. A recovery construct therefore cannot be an
  ordinary function: its first argument must be evaluated under the
  construct's control.

## Scope

1. One construct that evaluates `expr`; when `expr` fails with a recoverable
   error, it evaluates and returns `fallback`. When `expr` succeeds, `fallback`
   is not evaluated.
2. A written definition of which errors are recoverable. The starting
   proposal:
   - recoverable: evaluation failures of a fallible function, including
     `dm.data.*` failures and file-reference failures (missing, unreadable,
     fetch failure);
   - never recoverable: parse errors, passive call-check rejections (they are
     reported before anything runs), iteration-budget and other resource
     limits, and policy refusals that exist to stop a read (for example a
     remote host the fetch policy denies).
3. The same behavior on every expression surface: body interpolation,
   frontmatter values, block and transclusion conditions, callbacks, and
   lifecycle expressions.
4. The reference check treats a recovered failure as handled, so a guarded
   call no longer stops the document before commands run.
5. Static typing: the construct's type is the union of `expr`'s and
   `fallback`'s types, so a typed `load_data` result stays typed.
6. The catalog, the generated function documentation, and DMLS completion
   and hover know the construct.

## Non-goals

- Changing `||`, the ternary, or `try_frontmatter`. `try_frontmatter` stays
  as it is; whether to deprecate it is a later decision.
- Recovering from parse errors or anything the passive check rejects.
- A catch-all that hides programming errors such as a wrong argument count.

## Open questions

- **Spelling.** A function-shaped special form (`try(expr, fallback)`) or an
  operator. A function shape fits the catalog and DMLS, but needs a lazy
  first argument that the call binder does not support today.
- **Error access.** Whether the fallback can read the error (for example a
  local `error` holding the code and message), or only replace the value.
- **Reporting.** Whether a recovered failure leaves an advisory warning in
  the report, and whether `md compose --output json` lists it.
- **Read set and memo.** A recovered `load_data` read stays in the read set
  (the read happened). The memoized failure stays memoized, so a second
  unguarded call over the same file in the same command still fails; confirm
  that this is wanted.

## Acceptance criteria

- `try(load_data("bad.yaml", "string[]"), [])` composes to `[]` for a present
  but malformed file, and to the data for a valid one.
- `try(load_data("missing.yaml", "string[]"), [])` composes to `[]`, and the
  reference check does not fail on it.
- When `expr` succeeds, a side-effect-free probe in `fallback` shows it was
  never evaluated.
- A parse error inside `expr`, a passive call-check rejection, and an
  exceeded iteration budget still fail the document.
- Each expression surface listed in Scope has one test that recovers.
- `expression_type` of `try(load_data(f, "{ a: string }[]"), [])` is an array
  of `{ a: string }`.
- The `darkmatter/docs/` expression pages and the catalog document the
  construct and the recoverable-error list; the loading-data topic page shows
  it next to the `file_exists` pattern.

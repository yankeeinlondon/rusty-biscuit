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
depends-on:
    - 2026-11-04-loading-data
---

# Member Property Check

Report a read of a member that a value's known object type does not
declare, such as `h.nmae` when `h` is `{ name: string, port: number }`. The
check covers schema-typed frontmatter values and typed `load_data` results
alike, in composition and in DMLS.

## Motivation

A literal `as` contract now gives `load_data` results a static type, and
DMLS already offers `name` and `port` after `h.` inside a callback. The type
is known, but a misspelled member still evaluates to `null` and renders as
empty text with no report. 2026-11-04-loading-data excluded this check on
purpose: no member-level check exists for any value today, so adding one for
`load_data` alone would make two kinds of typed value behave differently.

## Current behavior

Verified against the code on 2026-11-07:

- **Only roots are checked.** `dm.expression.undeclared_property` is an
  advisory for an expression *root* that resolves to nothing: composition
  collects `UnknownRootCandidate`s and `unknown_identifiers::reconcile` drops
  roots that are keys of the final state, caller inputs, or declared at the
  top level of the effective schema. DMLS computes the same classification
  for roots (`providers/dsl.rs`). `{{ sever.name }}` warns; `{{ server.nmae }}`
  does not.
- **A missing member is `null`.** `evaluate_member`
  (`compose/expression/mod.rs`) returns `null` for a member an object lacks,
  with no observation.
- **The type is already available.** `TypeScope` types a variable or constant
  path from the document's effective schema, and
  `call_check::expression_type` types a `load_data` call with a literal `as`
  as that contract's shape (named references `unknown`). `map` and `filter`
  give their callback locals the element type. DMLS member completion after
  `h.` reads this typing.
- **Absence handling exists for roots.** A root read as a fallback primary,
  a ternary condition, a direct argument to a `| null` parameter, or under a
  branch fact that proves it present is not reported. The same rules do not
  yet apply to members, because members are never observed.

## Scope

1. Observe each member read `base.name` (and each step of a dotted path)
   whose base has a known static type. Report the read when **every**
   non-null arm of that type is an object type that declares its members
   and none of them declares `name`.
2. Never report when any arm is `unknown`/`any`, bare `object`, a dictionary
   (`{ <string>: T }`), or a named reference the static type left unresolved.
3. Apply the root rules for intentional absence: a handled read (fallback
   primary, ternary condition, `| null` argument, a branch fact) is not
   reported.
4. Report at the member's source span, as an advisory warning that never
   fails composition, once per distinct path per source document.
5. Composition and DMLS report the same reads, from one shared
   classification in the library.
6. Frontmatter values typed by the effective schema, callback locals typed
   by a literal `load_data` contract, and `::map`/`::filter` item locals are
   all covered.

## Non-goals

- Making a misspelled member an error, or changing its runtime value.
- Checking members of computed `as` contracts, whose type is `any`.
- Suggesting a spelling (a later DMLS quick fix could reuse the declared
  member list).

## Open questions

- **Code.** Reuse `dm.expression.undeclared_property` with a `path` that
  names the member, or add `dm.expression.undeclared_member`. A separate code
  lets a host filter the new advisories while they settle.
- **Unions.** Whether a member declared by only some object arms should be
  reported as "not declared by every arm", or stay silent as in Scope (1).

## Acceptance criteria

- This body expression reports one advisory at `nmae`, in composition and
  in DMLS, and still composes:

  ```markdown
  {{ map(load_data("hosts.yaml", "{ name: string, port: number }[]"), h => h.nmae) }}
  ```

- With a frontmatter schema declaring `server: { name: string }`,
  `{{ server.nmae }}` reports the same advisory; `{{ server.name }}` does not.
- `{{ server.nmae || "none" }}` and `{{ server.nmae ? 1 : 0 }}` report nothing.
- A value typed `{ <string>: string }`, `object`, `unknown`, or through an
  unresolved named import reports nothing for any member.
- `h.nmae` with a computed `as` reports nothing.
- The undeclared-property tests for roots pass unchanged.
- The `darkmatter/docs/` page that documents undeclared properties describes
  the member check.

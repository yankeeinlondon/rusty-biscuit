---
status: draft
created: 2026-09-13
clarified: true
needs_rulings: false
clarified_by: claude/opus
area: darkmatter
packages:
  - darkmatter
  - dmls
amends:
  - ../../features/_completed/2026-07-08-single-sourcing-schema/spec.md
related:
  - ../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md
  - ../../docs/topics/context-variables.md
  - ../../docs/inline/fm-interpolation.md
---

# One Array Rendering, and It Is JSON

## Summary

Darkmatter renders a bare array two different ways depending on which path
the value travels. `scalar_string` renders it as JSON text; the
interpolation output boundary renders it newline-joined. Both are reachable
from the same document, and nothing in the language tells an author which one
they are getting.

This fix picks one. A bare array renders as **JSON**, on every path. The
newline-joined form remains available, as it already is, through the explicit
`as_line_separated(…)` function, and two new functions —  `as_json(…)` and
`as_json5(…)` — join the list-rendering family so the new default has an
explicit spelling too.

This is a breaking change to rendered output, and it reverses a decision that
was ratified and shipped.

## Why This Matters Now

The split was found while specifying
[the nested-span fix](../../../claudine/fixes/2026-09-13-better-static-analysis/spec.md),
which mechanically rewrites `"text {{ span }}"` into `"text " + span`. That
rewrite is only faithful if `+` and interpolation render the same value the
same way. For every type but an array they do. For an array they do not, so
the rewrite silently changes a document's output.

That fix works around the split by **suppressing its rewrite suggestion for
array-valued spans**: the defect is still reported, but no fix is offered,
because a wrong suggestion is worse than none. Lifting that suppression is
part of this fix's scope. Until this lands, an author whose literal
interpolates an array is told the construct is broken and is not offered a
way to correct it.

The split is worth closing on its own merits regardless. Two renderers for
one type, chosen by which code path the value happened to take, is a defect
in the language rather than a feature of it.

## The Two Renderers

Both live in `darkmatter/lib/src/markdown/compose/expression/mod.rs`:

- **`scalar_string`** renders `Array` and `Object` as their JSON text. It is
  what the `+` operator calls, and what equality comparison and frontmatter
  shell expansion call.
- **`interpolation_output_string`** is identical to `scalar_string` except
  that a top-level array is rendered by joining its elements with `\n`. It is
  what the interpolation output boundary calls — three call sites, all in
  `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`.

So `{{ list }}` in a document body yields `a\nb`, while `{{ "" + list }}`
yields `["a","b"]`, from the same value in the same document.

## Reversing a Ratified Decision

The newline-joined default is not an accident or an oversight. It was
ratified as **D4 of
`darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md`**,
which introduced the list-rendering function family and chose newline-joining
as the bare-array default — `{{ items }}` deliberately made equivalent to
`{{ as_line_separated(items) }}`. `interpolation_output_string`'s own doc
comment cites "spec D4 default" as its justification.

This fix overturns that decision. The reasoning:

- **JSON is the honest default for a structured value.** Newline-joining
  renders an array as though it were prose, which reads well in the one case
  the original decision had in mind and misleads in every other. An author
  who interpolates an array without saying how it should look is better
  served by output that shows the value's shape than by output that hides it.
- **One value must not have two renderings.** Whatever default is chosen,
  having it depend on which operator the value passed through is not
  defensible. The original decision did not create the split — `scalar_string`
  predates it — but it widened it.
- **The explicit form already exists.** `as_line_separated(…)` shipped with
  D4 and is unchanged here, so nothing becomes unexpressible. The migration
  is a one-word edit, not a rewrite.
- **The rewrite generator needs a single answer.** A mechanical rewrite
  between the two forms cannot be correct while the two forms disagree.

**Obligation:** the PR description states the reversal in plain terms —
that this changes D4 of the single-sourcing-schema feature — so the decision
is discoverable from the history and not only from this file.

## Design

### D1. Unify on JSON, and delete the second renderer

The interpolation output boundary renders a top-level array as JSON, which
makes `interpolation_output_string` byte-for-byte identical to
`scalar_string`. It is **deleted** rather than kept as an alias, and its three
call sites in `compose/interpolation/evaluator.rs` call `scalar_string`
directly.

Keeping it as a thin wrapper would preserve the idea that the interpolation
boundary has its own rendering policy. It does not, after this change, and a
function that exists only to assert a distinction that no longer holds is how
the split would grow back.

### D2. Extend the list-rendering family

The family currently holds six functions, registered in
`expression/functions/collections.rs` with implementations in
`expression/functions/mod.rs`:

`as_line_separated`, `as_csv`, `as_tsv`, `as_space_separated`,
`as_unordered_list`, `as_ordered_list`.

Add two:

- **`as_json(list)`** — JSON text. The explicit spelling of the new default,
  exactly as `as_line_separated` was the explicit spelling of the old one.
- **`as_json5(list)`** — JSON5 text.

Both follow the family's existing conventions rather than inventing their
own: the same canonical-name-plus-alias registration, the same behavior for
an empty list, the same passthrough for a `null` argument, and the same error
for a non-list argument. Those behaviors are already pinned by the family's
tests and the new functions are held to them.

### D3. Lift the suggestion suppression in the nested-span fix

The nested-span fix suppresses its `+` rewrite suggestion when a flagged
literal contains an array-valued span, because the rewrite would not be
faithful. Once both paths render arrays identically the rewrite **is**
faithful, so this fix removes the suppression and the suggestion is offered
for array-valued spans like any other.

This is the only part of this fix that reaches into another package's code.
It is here rather than there because the suppression exists solely to
tolerate the split this fix closes, and leaving it behind would mean shipping
a permanent workaround for a temporary problem.

### D4. Documentation

Five locations assert the newline-joined default today. All of them are wrong
the moment this lands:

- `darkmatter/docs/topics/context-variables.md` — the note that a bare
  `{{ ctx.foo }}` renders an array line-separated, one element per line;
- `darkmatter/docs/inline/fm-interpolation.md` — the guidance to use an
  explicit function "or rely on the default line-separated" rendering;
- `darkmatter/docs/topics/darkmatter-expressions.md` — the List Formatting
  table row describing `as_line_separated` as "the default bare-array
  rendering";
- `darkmatter/docs/schemas/expression-functions.yaml` — the same description
  on the `as_line_separated` entry, plus new entries for `as_json` and
  `as_json5`;
- `.claude/skills/darkmatter/compose.md` — the same claim in the skill.
  Hash update via `md hash`.

Additionally:

- **The stale doc comment is corrected.**
  `interpolation_output_string`'s doc comment cites "spec D4 default" and
  describes the equivalence `{{ ctx.some_list }}` ≡
  `{{ as_line_separated(ctx.some_list) }}`. It goes with the function rather
  than being left pointing at a superseded decision.
- The equivalence comment on the interpolation evaluator's array-case test
  (`compose/interpolation/evaluator.rs`) makes the same claim and is re-cut.
- A migration note for authors, wherever the default is documented: a
  document that relied on newline-joined output moves to
  `as_line_separated(…)`, which is unchanged, or to whichever explicit
  function matches the intent — `as_unordered_list` for Markdown bullets,
  `as_csv` for a prose list.

## Breaking-Change Surface

Everything that changes behavior, enumerated so the review can be bounded:

| Surface | What changes |
| --- | --- |
| `compose/interpolation/evaluator.rs`, 3 call sites | Render arrays as JSON via `scalar_string` |
| `interpolation_output_string` | Deleted |
| `interpolation_output_string_renders_arrays_line_separated` | Re-cut to assert JSON, or removed with the function |
| `interpolation_output_string_matches_scalar_string_for_non_arrays` | Becomes unconditional; re-cut or removed |
| Evaluator array-case test doc comment | Re-cut; no longer asserts the `as_line_separated` equivalence |
| 5 documentation locations | Rewritten (D4) |
| Any document interpolating a bare array | Rendered output changes |

**Lifecycle output is on this boundary too.** Claudine's `say` and `message`
values render through the same interpolation output boundary, so this change
alters spoken and written lifecycle output for any prompt that interpolates a
bare array — not only document bodies. That is why this fix carries an
audit obligation and not merely a migration note for external authors: the
prompts in this repository are affected and are ours to fix.

**In-repo audit.** Every prompt and fixture in this repository that
interpolates a bare array is found and moved to an explicit function, with
the intended rendering chosen per site rather than by blanket substitution.
`prompts/commit.md` is one such site; **Ken is handling that file himself**,
moving `ctx.dirty_package_areas` to `as_csv(…)`, because a one-item array
renders as bare `claudine` under CSV, which is what the author meant. That
file is also a regression fixture for the nested-span fix at its pre-fix
commit, and this change does not disturb that.

## Scope

In scope:

- D1 through D4 above.
- Deleting `interpolation_output_string` and repointing its call sites.
- `as_json` and `as_json5`.
- Lifting the array-span suggestion suppression in the nested-span fix.
- The in-repo prompt and fixture audit.

Out of scope:

- Changing `scalar_string` itself, or the rendering of objects, which is
  already JSON on both paths and is not in dispute.
- Changing equality comparison or frontmatter shell expansion, which already
  call `scalar_string` and are unaffected.
- Adding a configuration switch for the default. One rendering is the point
  of the fix; a switch would reintroduce the ambiguity under a different
  name.
- Any of the static-analysis work in the related fix. The only coupling is
  D3's suppression lift.

## Testing

Every new test must fail on the commit before its fix, and the verification
record must say how that was shown. Tests that change an existing assertion
are named in the verification record as intentional changes, not treated as
regressions to be explained away.

### Darkmatter L1 (`darkmatter/lib`)

- `{{ some_list }}` in a document body renders JSON, not newline-joined
  text.
- The `+` path and the interpolation path render the same array identically,
  asserted directly rather than inferred from both matching a literal.
- `{{ some_object }}` is unchanged, confirming the fix touched arrays only.
- `as_json(list)` and `as_json5(list)` render as expected, and match the
  family's existing conventions for an empty list, a `null` argument, and a
  non-list argument — the cases `as_csv` and `as_unordered_list` already
  pin.
- `as_line_separated(list)` is unchanged and still produces newline-joined
  text, so the migration path is real rather than nominal.
- A one-item array renders as bare text under `as_csv`, which is the
  `prompts/commit.md` case.
- `interpolation_output_string_renders_arrays_line_separated` and
  `interpolation_output_string_matches_scalar_string_for_non_arrays` are
  re-cut or removed with the function, and the verification record names
  them.

### Cross-fix (with the nested-span fix in place)

- The rewrite of an array-valued span is byte-identical to the interpolation
  of the same span.
- The nested-span diagnostic on an array-valued span now **offers** a
  suggestion, where before this fix it offered none. This is the test that
  proves the suppression was lifted rather than forgotten.

### Evidence

macOS L1 locally through `just test`. Linux, native Windows, and WSL2
through CI on the PR.

## Acceptance Criteria

1. A bare array renders as JSON on both the `+` path and the interpolation
   path, and the two are asserted equal rather than separately asserted
   against a literal.
2. `interpolation_output_string` no longer exists, and its three call sites
   in `compose/interpolation/evaluator.rs` call `scalar_string`.
3. `as_json` and `as_json5` exist alongside the other six list functions and
   honor the family's conventions for empty, `null`, and non-list arguments.
4. `as_line_separated` is unchanged and still produces newline-joined text.
5. The array-span suggestion suppression in the nested-span fix is removed,
   and an array-valued span is offered a faithful rewrite.
6. Every in-repo prompt and fixture that interpolated a bare array has been
   audited and moved to an explicit function, with no document's rendered
   output changing unintentionally.
7. All five documentation locations are corrected, the stale "spec D4
   default" doc comment is gone, the evaluator test's equivalence comment is
   re-cut, and skill hashes are refreshed with `md hash`.
8. The PR description states that this reverses D4 of
   `darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md`.
9. L1 green on macOS locally and on Linux, Windows, and WSL2 in CI.

## Sequencing

This fix and the nested-span fix are independent and can land in either
order, which is the reason they are two documents.

- **If this lands first**, the nested-span fix ships without its suppression
  rule, and D3 here becomes a no-op that should be dropped from the plan
  rather than implemented against code that never existed.
- **If the nested-span fix lands first**, it ships the suppression, authors
  briefly get a diagnostic without a suggestion for array-valued spans, and
  D3 here removes it.

Whichever order is chosen, this fix should land as its own commit carrying
the reversal in its message, so it can be reverted without taking the static
analysis with it.

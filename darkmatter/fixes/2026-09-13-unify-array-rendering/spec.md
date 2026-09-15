---
status: draft
created: 2026-09-13
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-15
review_iterations: 1
clarified: true
needs_rulings: false
clarified_by: claude/opus
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine
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

This fix picks one string representation. A bare array rendered into text is
compact **JSON** on every expression string-output path. Typed boundaries stay
typed: an exact whole-value frontmatter interpolation and a typed dynamic
sequence source continue to return an array rather than a JSON string. The
newline-joined form remains available, as it already is, through the explicit
`as_line_separated(…)` function. Two new functions — `as_json(…)` and
`as_json5(…)` — join the list-rendering family; `as_json(…)` is the explicit
spelling of the new default.

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
  called at three sites in
  `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`: the
  presentation-value fast path, the bare-variable array arm, and the general
  evaluated-expression path.

So `{{ list }}` in a document body yields `a\nb`, while `{{ "" + list }}`
yields `["a","b"]`, from the same value in the same document.

That contrast applies when the value is being embedded into text. It does not
apply to `eval_json` or another typed whole-value boundary, which must not call
either string renderer merely to satisfy this fix.

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

`scalar_string` itself is unchanged. This is a deliberately narrow redirect:
its graph has direct consumers in Darkmatter expression evaluation and
frontmatter shell expansion and in Claudine lifecycle, loop, sequence, and
dispatch rendering. None of those existing calls is rewritten, and their
object, scalar, and array behavior remains byte-identical. The evaluator's
object-specific `get_string` hook is also preserved; this fix replaces only
the array arm's renderer and the two generic output calls.

Nor does this redirect widen a string boundary. `eval_json`, exact whole-value
frontmatter interpolation, loop mutations that preserve a typed single span,
and typed dynamic sequence sources continue to return `Value::Array`. Only a
caller that already requested text receives compact JSON.

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

- **`as_json(list)`** — compact strict JSON produced by the same
  `scalar_string` path as the new bare-array default. For every array `x`,
  `as_json(x)` and bare interpolation of `x` are byte-identical. In
  particular, `as_json([])` returns `"[]"`, strings use JSON double quotes and
  escapes, nested arrays and objects retain their structure, and no trailing
  newline is added.
- **`as_json5(list)`** — compact, single-line, idiomatic JSON5 produced by
  `biscuit_file::json5::to_json5_compact`. It uses that repository authority
  rather than adding a Darkmatter-local serializer. In particular,
  `as_json5([])` returns `"[]"`; strings use the formatter's single-quoted
  representation; eligible object keys are unquoted; and no trailing newline
  is added.

Both follow the family's existing argument conventions: exactly one argument,
`null` propagates as `Value::Null`, and a non-array argument returns the
standard `requires an array argument` error. Their canonical registrations
carry the standard collapsed aliases `asjson` and `asjson5`. They are appended
to the List Formatting catalog with the next free orders, 97 and 98, so the
six shipped entries retain their relative and numeric order. The
typed-signature and runtime-registration parity tests must cover all eight
functions.

Darkmatter already depends on `biscuit-file`, whose default feature set
currently enables JSON5. Because Darkmatter now calls the JSON5 formatter
directly, its dependency declaration explicitly includes the `json5` feature;
the implementation must not rely on that feature remaining in
`biscuit-file`'s defaults. This adds no crate and no second JSON5
implementation.

> **Reader's note:** an earlier draft said the new functions inherited the
> family's empty-list behavior. That would make `as_json([])` return an empty
> string while the default returns `[]`, contradicting the purpose of an
> explicit spelling. JSON and JSON5 therefore preserve the empty container;
> only the argument-validation and null-propagation conventions are shared
> with the joiners.

### D3. Retire the aggregate suggestion suppression in the nested-span fix

The nested-span fix suppresses its `+` rewrite suggestion when a flagged
literal contains a span declared as an array or object. Arrays motivated the
rule because their two renderers disagree today; objects were conservatively
grouped with them even though both current renderers already produce JSON.
Once the array paths agree, no aggregate type requires special treatment, so
this fix removes the entire declared-array-or-object suppression and its type
lookup plumbing that exists only for that suppression. Suggestions are offered
for declared arrays and objects just as they already are for scalars and
untyped values. Tests pin both aggregate cases so the object half is not left
behind as dead policy. Type information used by any other diagnostic remains.

This is the only part of this fix that reaches into the related nested-span
implementation across Darkmatter, DMLS, and Claudine. It is here rather than
there because the suppression exists solely to tolerate the split this fix
closes, and leaving it behind would mean shipping a permanent workaround for
a temporary problem. Its implementation and documentation are removed at the
same landing boundary; the related spec remains the historical record of why
the interim suppression existed.

### D4. Documentation

Six active authored locations assert or demonstrate the newline-joined default
today. All of them are wrong the moment this lands:

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
- `darkmatter/features/_completed/2026-07-08-single-sourcing-schema/examples/as_line_separated.yaml`
  — the shipped example's claim that the function is equivalent to bare-array
  interpolation;
- `.claude/skills/darkmatter/compose.md` — the same claim in the skill.

The completed D4 specification, its plan, and its review records remain
unchanged historical evidence. This spec's `amends` relationship, the updated
live documentation, and the PR description record the reversal without
rewriting the original decision after the fact.

Additionally:

- **The stale doc comment is corrected.**
  `interpolation_output_string`'s doc comment cites "spec D4 default" and
  describes the equivalence `{{ ctx.some_list }}` ≡
  `{{ as_line_separated(ctx.some_list) }}`. It goes with the function rather
  than being left pointing at a superseded decision.
- The equivalence comment on the interpolation evaluator's array-case test
  (`compose/interpolation/evaluator.rs`) makes the same claim and is re-cut.
- The active capture comment in
  `compose/context/capture/repo.rs` and the L2 chooser comment in
  `claudine/cli/tests/level2_auto_complete_chooser.rs` are corrected. A final
  repository search for `bare array`, `bare-array`, and `line-separated by
  default` must leave only historical records or unrelated uses, each reviewed
  rather than silently excluded.
- `darkmatter/docs/dependencies.md` records that Darkmatter directly uses
  `biscuit-file`'s JSON5 formatter and explicitly enables its `json5` feature.
- A migration note for authors, wherever the default is documented: a
  document that relied on newline-joined output moves to
  `as_line_separated(…)`, which is unchanged, or to whichever explicit
  function matches the intent — `as_unordered_list` for Markdown bullets,
  `as_csv` for a prose list.
- After the skill edit, refresh `.claude/skills/darkmatter/compose.md` with
  `md hash`; do not hand-edit its hash.

## Breaking-Change Surface

Everything that changes behavior, enumerated so the review can be bounded:

| Surface | What changes |
| --- | --- |
| `compose/interpolation/evaluator.rs`, 3 call sites | Render arrays as JSON via `scalar_string` |
| `interpolation_output_string` | Deleted |
| `compose/expression/functions/mod.rs` | Add the two serializers and cover their exact empty, nested, escaping, null, arity, and type-error behavior |
| `compose/expression/functions/collections.rs` | Register `as_json`/`asjson` and `as_json5`/`asjson5` |
| `darkmatter/lib/Cargo.toml` | Explicitly enable the existing `biscuit-file/json5` feature |
| Expression-function catalog and generated table | Add both descriptors and executable examples; expand six-function assertions to eight |
| DMLS completion/hover | Expose both functions from the shared catalog and expand the all-formatters assertion to eight |
| `interpolation_output_string_renders_arrays_line_separated` | Removed with the function; its JSON obligation moves to evaluator integration tests |
| `interpolation_output_string_matches_scalar_string_for_non_arrays` | Removed with the function; unchanged scalar/object behavior is pinned at the evaluator boundary |
| Active source and test comments | Re-cut where they state the superseded default; historical decision records stay intact |
| Six authored documentation/example locations | Rewritten as specified by D4; the skill hash is refreshed |
| Nested-span implementation and tests | Remove the declared aggregate suppression and offer array/object rewrites |
| Any mixed-string or body interpolation of a bare array | Rendered output changes from newline-joined text to compact JSON |

**Some lifecycle output is on this boundary too.** An exact whole-value
lifecycle interpolation resolves to a typed value and is already stringified
through `scalar_string`, so it already emits JSON and does not change. A
`say`, `message`, or other lifecycle value that embeds an array interpolation
inside surrounding text reaches Darkmatter's mixed-string interpolation path
and does change. The same distinction applies to ordinary frontmatter. That
is why this fix carries an audit obligation rather than merely a migration
note for external authors: repository prompts can be affected, but typed
whole-value consumers must not be migrated as though they were prose.

**In-repo audit.** Every active prompt, example document, and test fixture in
this repository that embeds a bare array into text is found and moved to an
explicit function, with the intended rendering chosen per site rather than by
blanket substitution. The audit uses schema/catalog array types where
available and covers local frontmatter arrays as well as `ctx.*`; a text search
alone cannot infer the runtime type of an arbitrary expression. Exact
whole-value typed consumers are classified and retained, not converted to
formatters. Expected-output snapshots and prose surrounding every migrated
site are updated together.

`prompts/commit.md` is one affected site; **Ken is handling that file
himself**, moving `ctx.dirty_package_areas` to `as_csv(…)`, because a one-item
array renders as bare `claudine` under CSV, which is what the author meant.
That file is also a regression fixture for the nested-span fix at its pre-fix
commit, and this change does not disturb that. The implementation must preserve
Ken's edit and must not replace or reformat the file during the broader audit.

## Scope

In scope:

- D1 through D4 above.
- Deleting `interpolation_output_string` and repointing its call sites.
- Compact `as_json` and `as_json5`, their collapsed aliases, catalog entries,
  DMLS discovery, and explicit use of `biscuit-file`'s JSON5 feature.
- Removing the aggregate-span suggestion suppression in the nested-span fix.
- The in-repo prompt and fixture audit.

Out of scope:

- Changing `scalar_string` itself, or the rendering of objects, which is
  already JSON on both paths and is not in dispute.
- Changing equality comparison or frontmatter shell expansion, which already
  call `scalar_string` and are unaffected.
- Stringifying typed whole-value results. Exact frontmatter, loop, and sequence
  expression boundaries continue to preserve arrays.
- Adding a configuration switch for the default. One rendering is the point
  of the fix; a switch would reintroduce the ambiguity under a different
  name.
- Adding pretty-print controls to `as_json` or `as_json5`. Both are compact and
  deterministic in this fix; a configurable serializer is a separate feature.
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
- An exact whole-value frontmatter array stays typed, while the same array in
  a mixed frontmatter string renders as compact JSON. This prevents the fix
  from widening the text boundary.
- `as_json(list)` is byte-identical to bare-array text rendering for empty,
  scalar, mixed, and nested arrays. Escaped quotes, backslashes, newlines, and
  Unicode exercise the serializer rather than only simple ASCII values.
- `as_json5(list)` is compact JSON5 for empty, scalar, mixed, and nested arrays;
  a nested object proves the `biscuit-file` formatter's single quotes and
  eligible unquoted keys are used. Its output parses back to the original
  `serde_json::Value` through the repository JSON5 parser.
- Both new functions propagate `null`, reject a non-array, and reject wrong
  arity consistently with the family. Their empty-array result is `[]`, not
  the joiners' empty string.
- `as_line_separated(list)` is unchanged and still produces newline-joined
  text, so the migration path is real rather than nominal.
- A one-item array renders as bare text under `as_csv`, which is the
  `prompts/commit.md` case.
- `interpolation_output_string_renders_arrays_line_separated` and
  `interpolation_output_string_matches_scalar_string_for_non_arrays` are
  re-cut or removed with the function, and the verification record names
  them.
- Runtime registration/catalog parity covers `as_json`/`asjson` and
  `as_json5`/`asjson5`; the typed list-formatter assertion covers all eight
  catalog entries and their executable examples.

### Darkmatter CLI and DMLS L1

- A deterministic `md compose` integration test launched through
  `CliProcessFixture` proves a body interpolation emits compact JSON while
  `as_line_separated` remains newline-joined. The fixture pins its environment
  and does not use a raw process spawn.
- DMLS completion contains `as_json` and `as_json5`, and hover reports their
  catalog-backed typed signatures and descriptions. The existing
  all-formatters completion test expands from six functions to eight.

### Claudine L1

- Exact whole-value dynamic sequence sources and single-span loop mutations
  stay typed arrays, while mixed strings use compact JSON. These regressions
  guard the existing typed/string boundary without changing sequence or loop
  semantics.
- Exact whole-value lifecycle communication fields keep their existing compact
  JSON output; lifecycle values that embed an array inside surrounding text
  move from newline-joined output to compact JSON unless the in-repo audit
  assigns an explicit formatter.

### Cross-fix (with the nested-span fix in place)

- The rewrite of an array-valued span is byte-identical to the interpolation
  of the same span.
- Nested-span diagnostics on declared array- and object-valued spans now
  **offer** suggestions, where the interim rule offered none. These tests prove
  the entire aggregate suppression was removed rather than only its motivating
  array branch.
- Claudine's affected lifecycle diagnostic/quick-fix path agrees byte-for-byte
  with Darkmatter and DMLS after the suppression removal.

### Evidence

Run `just test` from both `darkmatter/` and `claudine/` on macOS. Linux, native
Windows, and WSL2 run the affected Darkmatter and Claudine L1 cells through CI
on the PR. No L2 or browser behavior changes; the corrected chooser comment is
covered by the existing L2 suite but does not require rerunning that tier for a
comment-only edit.

## Acceptance Criteria

1. A bare array renders as JSON on both the `+` path and the interpolation
   path, and the two are asserted equal rather than separately asserted
   against a literal.
2. `interpolation_output_string` no longer exists, and its three call sites
   in `compose/interpolation/evaluator.rs` call `scalar_string`.
3. `as_json` and `as_json5` exist alongside the other six list functions and
   have the exact compact serialization contracts in D2. Both return `[]` for
   an empty array, propagate `null`, and reject non-arrays and wrong arity.
4. `as_line_separated` is unchanged and still produces newline-joined text.
5. `as_json5` delegates to `biscuit_file::json5::to_json5_compact`, and
   Darkmatter explicitly enables the existing `biscuit-file/json5` feature.
6. Typed whole-value frontmatter, loop, and sequence results remain arrays;
   only pre-existing text boundaries stringify them.
7. The declared array-or-object suggestion suppression in the nested-span fix
   is removed, and both aggregate types are offered faithful rewrites.
8. Every active in-repo prompt, example, and fixture that embeds a bare array
   in text is audited and moved to an explicit function, with expected output
   changing only where documented. Typed whole-value uses are retained.
9. All six authored documentation/example locations and all active source/test
   comments are corrected, historical D4 records remain intact, and the
   Darkmatter skill hash is refreshed with `md hash`.
10. Runtime registration, the expression catalog, generated documentation,
    DMLS completion/hover, and their parity tests all expose the two canonical
    names and collapsed aliases consistently.
11. The PR description states that this reverses D4 of
   `darkmatter/features/_completed/2026-07-08-single-sourcing-schema/spec.md`.
12. Darkmatter and Claudine L1 are green on macOS locally and on Linux, native
    Windows, and WSL2 in CI.

## Sequencing

This fix and the nested-span fix are independent and can land in either
order, which is the reason they are two documents.

- **If this lands first**, the nested-span fix ships without its suppression
  rule, and D3 here becomes a no-op that should be dropped from the plan
  rather than implemented against code that never existed.
- **If the nested-span fix lands first**, it ships the suppression, authors
  briefly get a diagnostic without a suggestion for declared array- or
  object-valued spans, and D3 here removes that interim rule.

Whichever order is chosen, this fix should land as its own commit carrying
the reversal in its message, so it can be reverted without taking the static
analysis with it.

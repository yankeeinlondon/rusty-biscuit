---
status: "ready for planning and implementation"
reviewed: true
review_iterations: 2
---

# Lazy-by-default `file` references, with an opt-in `eager` constraint

## Problem

Darkmatter's `file` schema type is **eager**: at compose/validate time it
resolves a `file`-typed frontmatter value and **fails if the file does not
exist on disk**. That is correct for an *input* file (a spec/review the prompt
expects the caller to provide) but wrong for an *output* file (a path the prompt
itself is about to create).

The motivating case is `sniff/prompts/plan-review-implementation.md`:

```yaml
$schema:
    review: file(required; match(**/*review*.md))   # INPUT — should exist
    plan: file                                       # OUTPUT — created by this run
    iteration: number
plan: "{{dirname(review)}}/plan-{{iteration}}.md"
```

`plan` names the plan file this run produces. It cannot exist at compose time —
nor even when the agent launches — yet `plan: file` rejects it with
*"no existing file matched reference … plan-1.md"*. There is no way to express
"this is a file path, but do not require it to exist yet."

A `file` reference is, by nature, **lazy**: it is a path, not a guarantee of
presence. The existing `file_exists(var)` function (and ternary guards) already
exist precisely so an author can *test* presence where they need to. The schema
type should default to that lazy posture and let the author **opt in** to eager
existence validation for the cases that warrant it.

## Proposal

Make the `file` schema type **lazy by default** and add an **`eager`**
constraint that restores compile-time existence validation.

```yaml
$schema:
    foo: file                      # lazy + optional   (default)
    foo: file(required)            # lazy + required   (must be PRESENT, need not EXIST)
    foo: file(eager)               # eager + optional  (if present, must EXIST)
    foo: file(eager; required)     # eager + required  (must be PRESENT and EXIST)
    foo: file(eager)[]             # eager applies to each present array item
```

`eager` and `required` are **orthogonal** flags:

- **`required`** — the property must be **present** (non-null) in the
  effective frontmatter. (Existing meaning, unchanged.)
- **`eager`** — *if the property has a value*, that value must resolve to an
  **existing file**; a present-but-unresolvable reference is fatal (current
  `file` behavior). Absent unless explicitly requested.

`eager` is **optional by default**: `file(eager)` validates existence only when
a value is present; an absent/null optional `file(eager)` is fine.

For `file[]`, laziness/eagerness is an **item-level** constraint because each
array element is a file reference. The author writes `file(eager)[]`; array
constraints still follow `[]` (`file(eager)[](min(1); unique)`). The array
property's own presence is still governed by `required` exactly as today.

### Semantics matrix

For a `file`-typed property `P` with effective value `V`:

| Declaration               | `V` absent / null        | `V` present                                              |
|---------------------------|--------------------------|----------------------------------------------------------|
| `file`                    | OK (optional)            | OK — any well-formed reference; **existence not checked** |
| `file(required)`          | **error** (missing req.) | OK — well-formed reference; **existence not checked**     |
| `file(eager)`             | OK (optional)            | must resolve to an **existing** file, else fatal          |
| `file(eager; required)`   | **error** (missing req.) | must resolve to an **existing** file, else fatal          |

"Well-formed reference" means `biscuit_file::FileReference::new(value)` parses
(a syntactically valid path / `path:line:col` reference). A malformed reference
string is an error under **both** lazy and eager — laziness defers *existence*,
not *syntax*.

Because `FileReference::new()` is construction-only, lazy validation must not
call `resolve()`, `resolve_from()`, `resolve_file_ref_with_fallback()`, or any
helper that can touch the filesystem, inspect git state, expand missing
environment variables, or require vault configuration. Lazy `file` accepts a
syntactically valid `{{MISSING_ENV}}/out.md`, `vault:future.md`, `%@future.md`,
or `./future.md`; those only become fatal if a later read-side function or an
`eager` schema constraint actually resolves them.

### `match(...)` is metadata, not validation

`match(...)` is **never** a validation constraint. It is **metadata**: a set of
glob patterns that the suggestion surfaces — shell completion and the
interactive missing-property chooser — read to present a context-aware set of
candidate files. A `file(match('*.png'))` value is **never rejected** for
failing to match the globs; the globs only shape what completion *offers*.

The only validation a `file` type performs is therefore:

1. **syntax** — the value parses as a `FileReference` (both lazy and eager);
2. **presence** — `required` (must be non-null);
3. **existence** — `eager` (if present, must resolve to an existing file).

`match` participates in **none** of these. It is orthogonal to `eager`.

> **Drift note — current code validates `match`; intent says it must not.**
> Today `format.rs::DarkmatterMatchKeyword::validate` returns
> `ValidationError::custom("… does not match the configured file globs")` for an
> *existing* file that fails the globs (it defers only when the file is missing).
> That validation contradicts the metadata-only intent and must be **removed**
> as part of this work. This is safe and self-contained because completion and
> the interactive chooser read the patterns from the **simplified** schema
> (`Constraint::Match` → `CompletionKind::File { patterns }` in
> `schemas/completion.rs`, and `InteractiveShape::File` in claudine's
> `schema_validation.rs`) — **not** from the compiled JSON-schema keyword. So the
> `x-darkmatter-match` JSON-schema keyword and its validator can be dropped with
> zero impact on the suggestion surfaces.

## Scope boundary — what this does and does NOT change

This spec changes **one thing**: the **SimplifiedSchema `file` type's default
existence posture**. Bare `file` becomes lazy/syntax-only; `file(eager)` keeps
the existing existence check.

It explicitly does **not** touch the orthogonal, already-ratified **read-side
expression-function fatality** (`frontmatter(x)`, `link(x)`, `absolute(x)`,
`relative(x)`, transclusion, …). Those functions *actively use* the file (read
its frontmatter, render a link, inline its body), so they must resolve a real
file at the moment they run, and a present-but-unresolvable reference stays
**fatal** — governed by `ExpressionError::is_authoring_fatal`
(`darkmatter/lib/src/markdown/compose/expression/error.rs`) and the fatality
matrix in `…/compose/interpolation/fatality_characterization.rs`. The guard for
those remains `file_exists(...)` / a ternary, or — for lifecycle events — DM1
deferral so the function only runs at event time. See the
[`2026-06-28-real-errors`](../2026-06-28-real-errors/spec.md) decision
*"Missing file references are fatal in lenient compose mode."*

Concretely, in the motivating prompt the `success` event's
`frontmatter(plan, 'total_phases')` is **not** in scope here: it is a read-side
call that is correctly deferred to event time (when the plan exists). This spec
only makes the top-level `plan: file` schema declaration tolerate a not-yet-
existing path.

## Why this is the right shape

- It matches the author's mental model: a `file` value is a *reference*, and you
  *opt in* to "must already exist," rather than opting out of an over-eager
  default.
- It needs no new vocabulary beyond one constraint keyword that reads naturally
  alongside `required` and `match(...)`.
- The eager path is exactly today's behavior, so the win is additive: existing
  diagnostics, the launch-area-fallback resolution order, and the faceted
  `composition.invalid_file_reference` error are all reused unchanged for
  `eager`.

## Implementation surface

Change points (file references are point-in-time anchors; trust the symbol
names). Grouped by the touch-points mapped from the code.

### Darkmatter (the bulk)

1. **Constraint enum** — add `Constraint::Eager` to
   `schemas/simplified/types.rs` (the `Constraint` enum, ~L211). Update the
   constraint-name display arm (~L270) to `"eager"`.

2. **Parser** — recognize the bare `eager` keyword in the `file(...)` constraint
   list: `schemas/simplified/grammar.rs::parse_one_constraint` (~L1007), a new
   arm `("eager", false) => Constraint::Eager` beside the `required` arm
   (~L1028). **`eager` is valid ONLY on `file` (D2).** Applied to any other type
   (`string(eager)`, `number(eager)`, …) it is an **immediately fatal** schema
   error — surfaced at the same per-type constraint-compatibility check that
   already rejects other mis-applied constraints (e.g. the
   `other => return Err(invalid_constraint(name, …))` arms in the non-`file`
   fragment builders in `convert.rs`). Prefer the earliest stage that has the
   type in hand; the error must abort schema preparation, not warn.

3. **Serializer** — round-trip `Constraint::Eager` back to `eager` in
   `schemas/simplified/serialize.rs::write_constraint` (~L107).

4. **JSON-Schema lowering** — `schemas/simplified/convert.rs::file_fragment`
   (~L548). When the constraints contain `Eager`, emit the **eager** form;
   otherwise the **lazy** form. **Mechanism (revised by review, see D1):** keep
   the established raw JSON Schema format tag `format: darkmatter-file` as the
   eager/existence-checking validator, and add a new lazy tag,
   `format: darkmatter-file-reference`, for SimplifiedSchema's default `file`.
   `file(eager)` lowers to `darkmatter-file`; bare `file` lowers to
   `darkmatter-file-reference`. This keeps the existence decision a build-time
   fact the format closure can read, since a `jsonschema` *format* closure
   cannot see sibling keywords.

   Reader's note: the draft originally proposed flipping `darkmatter-file` to
   lazy and adding `darkmatter-file-eager`. That would have made the
   SimplifiedSchema authoring surface tidy, but it would silently change the
   established raw JSON Schema contract documented and tested today:
   `format: darkmatter-file` means
   "parse and resolve to an existing file." Keeping that tag eager avoids an
   accidental standard break while still delivering the intended author-facing
   SimplifiedSchema change.

   **Also: stop emitting `x-darkmatter-match`** — `match` is metadata, not
   validation (see above), so the compiled JSON Schema no longer carries it. The
   `Constraint::Match` arm in `file_fragment` is dropped; the patterns survive on
   the simplified-schema atom, which is where completion reads them.

5. **The existence gate (crux)** — `schemas/format.rs`. Split the current
   `resolve_file_reference` existence behavior across the two tags:
   - `darkmatter-file-reference` (lazy): validate **syntax only** —
     `FileReference::new(value).is_ok()`. No resolve, no `!path.exists()`, no
     environment/vault/git lookup.
   - `darkmatter-file` (eager): the **current** full check — resolve via
     `resolve_file_reference` (document-first → launch-area fallback) and fail
     on `FileReferenceFailure::{Resolution, NoMatch}` exactly as today (L186–221).
   Both closures still capture the `base_dir`/`fallback` anchors the way the
   single closure does today (see `build_validator(&schema, base_dir, fallback)`
   and the closure-capture note in [[project_claudine_expr_resolution_launch_cwd]]).

6. **Remove the `match` validator** — delete `DarkmatterMatchKeyword`,
   `match_keyword_factory`, and the `x-darkmatter-match` keyword registration in
   `schemas/format.rs` (~L223–368), and stop registering it on the validator
   build (`build_validator`). `match` becomes simplified-schema metadata only.
   This also removes the `format == "darkmatter-file"` co-requirement check
   (~L241), so it does not need to learn the new eager tag.

7. **Validation diagnostics** — update `schemas/validate.rs` so the targeted
   file-reference diagnostic substitution remains scoped to eager
   `darkmatter-file` failures only. Lazy `darkmatter-file-reference` should only
   fail on malformed syntax; its targeted message can reuse the
   `FileReferenceFailure::InvalidSyntax` wording, but it must not call
   `resolve_file_reference` to produce that message. This prevents a lazy-schema
   syntax error from accidentally performing the eager resolution work in the
   reporting path.

8. **Schema descriptor catalog and docs** — update
   `schemas/about.rs` and any generated context/schema-language reports:
   - `file` description: a file reference, lazy by default.
   - accepted constraints: `eager, match(glob, ...), default, required`.
   - `match` descriptor: metadata for suggestions, not validation.
   - new `eager` descriptor: `file` only, validates existence when a value is
     present.
   - JSON Schema effect: bare `file` lowers to
     `format: darkmatter-file-reference`; `file(eager)` lowers to
     `format: darkmatter-file`.

9. **Existing generated/coercion schemas** — audit hard-coded schema fragments
   in `schemas/coerce.rs`, tests, and docs that currently mention
   `format: darkmatter-file`. If they are modeling SimplifiedSchema bare `file`,
   switch them to `darkmatter-file-reference`; if they intentionally assert the
   raw eager format contract, leave them as `darkmatter-file` and make that
   intent explicit in the test name/comment.

### Claudine (consumption + its own prompts)

10. **No re-validation needed** — claudine does not independently check `file`
   existence; it trusts Darkmatter's validator
   (`claudine/lib/src/composition/schema_validation.rs`). The faceted error
   (`composition.invalid_file_reference`) and the
   `invalid_required`/`invalid_optional` categorization (driven by `is_required`,
   ~L715) are reused for the eager path with no change. `required` still means
   "present"; `eager` does not affect requiredness categorization.

11. **Completion / interactive chooser — no change.** `CompletionKind::File`
   (`schemas/completion.rs`) and `InteractiveShape::File`
   (`schema_validation.rs`) carry only `patterns`; eager/lazy is a
   validation-time concern, not a candidate-set concern.

## Migration & breaking-change impact

> Darkmatter/Claudine have **no installed user base**, so a default flip is
> acceptable (consistent with the `2026-06-28-real-errors` posture). Call it out
> rather than soften it.

Flipping the default from eager→lazy means **every existing `file` and
`file(required)` schema stops validating existence.** Authors who relied on the
eager check must add `eager`.

This breaking change is scoped to **SimplifiedSchema** (`file` / `file(required)`).
Raw JSON Schema authors who explicitly use `format: darkmatter-file` keep the
existing eager behavior. Raw JSON Schema authors who want lazy syntax-only
validation can opt into `format: darkmatter-file-reference`, but the preferred
authoring surface remains SimplifiedSchema.

The motivating prompt becomes:

```yaml
$schema:
    review: file(eager; required; match(**/*review*.md))   # INPUT — restore existence check
    plan: file                                             # OUTPUT — now lazy, passes
    iteration: number
```

> **Prompt audit is owner-managed (Ken), out of scope for implementation.**
> Existing prompts that declare a `file`/`file(required)` input and rely on
> existence validation must add `eager`; this sweep is not part of the
> Darkmatter/Claudine code change.

This is itself the validation that the design solves the reported bug: `review`
(an input that must exist) opts back into eager; `plan` (an output) is lazy and
no longer rejected.

## Test impact

These tests assert the **current eager-by-default** behavior and must be
re-pointed (existing case → `file(eager)`; add lazy companions asserting a
missing file is accepted):

- `schemas/format.rs::file_format_rejects_missing_file` (~L483) and its
  `…accepts_existing_file` sibling — split into lazy (accepts missing) vs eager
  (rejects missing).
- `schemas/validate.rs::darkmatter_file_match_missing_file_produces_one_file_reference_diagnostic`
  (~L1246) — under lazy this produces **zero** diagnostics; add an eager variant
  that keeps the single (existence) diagnostic.
- **Remove** any test asserting `match` *rejects* an existing non-matching file
  (match no longer validates). Add a test proving an existing file that does
  **not** match the globs still validates (eager passes on existence; match is
  metadata).
- New coverage: the full 4-cell matrix, the malformed-syntax-is-fatal-under-both
  case, an array-item case (`file(eager)[]` rejects a missing item while
  `file[]` accepts it), and a test that `Constraint::Match` patterns still reach
  `CompletionKind::File`/`InteractiveShape::File` after `x-darkmatter-match` is
  dropped from the compiled JSON Schema.
- **D2:** `eager` on a non-`file` type (`string(eager)`, `number(eager)`, …) is
  a fatal schema-preparation error — one test per representative type, asserting
  it aborts (not warns) and names the offending type/constraint.
- Raw JSON Schema compatibility: `format: darkmatter-file` still rejects a
  missing file, while `format: darkmatter-file-reference` accepts the same
  missing-but-syntactically-valid value and rejects only malformed syntax.
- Descriptor parity: `schemas/about.rs::constraint_set_matches_descriptor_set`
  is updated to include `Constraint::Eager`, and descriptor text no longer says
  `match` restricts accepted paths.
- A claudine end-to-end (`md compose` + `claudine compose`) test mirroring the
  motivating prompt: lazy `plan` composes; eager `review` still rejects a
  missing review.

## Decisions

- **D1 — eager compilation mechanism (revised by review).** Two `format` tags:
  `darkmatter-file-reference` is lazy/syntax-only and is emitted for bare
  SimplifiedSchema `file`; `darkmatter-file` keeps its established
  eager/existence semantics and is emitted for SimplifiedSchema `file(eager)`.
  SimplifiedSchema authors still write only `file` / `file(eager)`, while raw
  JSON Schema authors keep backward-compatible eager behavior for the existing
  `darkmatter-file` tag.
- **D2 — `eager` is `file`-only (locked).** It is a `file`-type constraint;
  applying it to any other SimplifiedSchema type is an immediately fatal schema
  error, never a warning (see Implementation §2).
- **D3 — prompt audit owned by Ken**, out of scope for the code change (see
  Migration).

## Non-goals

- Changing read-side expression-function fatality (`frontmatter`/`link`/
  transclusion). Out of scope; see the scope boundary above.
- A general "output file" lifecycle/declaration concept beyond the lazy `file`
  reference.
- Pure string-pattern validation of paths (that is `pattern(<regex>)`).

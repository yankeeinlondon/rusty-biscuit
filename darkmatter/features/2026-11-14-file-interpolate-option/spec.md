---
kind: feature
created: 2026-11-14
status: draft-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-14-edit-action
    - 2026-11-14-absent-equals-null
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
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: false
review_iterations: 0
---

# `::file interpolate=false`: transclude prose without evaluating it

## Why this exists

Caller skills (`{package-area}/docs/use-skills/`) are composed when they are
published, so they can transclude passages of the package's canonical
`docs/` instead of restating them. Those docs pages are written to be read
as they are, so they show expressions raw, for example
``Use `{{ name }}` to interpolate.``. Transcluded, that example is
evaluated: against the skill's own frontmatter if a name matches (the line
above became ``Use `s` to interpolate.`` in a test), otherwise to an
undeclared-name warning. Fenced code and `::code` content already stay
verbatim; inline prose does not, and `::file` has no way to say "bring this
passage in as written".

## Author rulings (2026-11-14)

- **One option, `interpolate: boolean(default(true))`.** Written on the
  directive as `interpolate=false`; absent (or `null`, per
  `2026-11-14-absent-equals-null`) means `true`, today's behavior.
- **Its scope is the transcluded document's prose only.** The child's
  frontmatter is always interpolated; there is no option to stop that.

```md
::file ../../topics/lifecycle/index.md interpolate=false
```

## Proposed details (confirm before planning)

1. **Only prose interpolation stops.** The child's directives still run:
   a nested `::file` or `::code`, and `::block when=…`, whose condition is a
   directive argument rather than prose.
2. **Written exactly as is.** With `interpolate=false`, every `{{ … }}` in
   the child's prose reaches the output as written, and a `{{{ … }}}`
   interpolation literal keeps all three braces (it is not reduced to two).
3. **Inherited.** A transclusion inside the child is also uninterpolated, so
   the whole passage stays as written; a nested `::file` may not turn it back
   on (`interpolate=true` there has no effect, or is a warning).
4. **Directive inputs are unchanged.** `set=`, `replace=`, `exclude=`, and
   the other `::file` options on the directive itself work as today; they
   are the parent's inputs, not the child's prose.
5. **Values.** `interpolate` accepts what `boolean` accepts under the
   conversion rules (`false`, `"false"`, `no`, `off`, `0`, …); anything else
   is a located directive error.

## Requirements

- Parsing, validation, composition, `md compose --shell` discovery, and DMLS
  (completion, hover, diagnostics for the option) all know the option.
- DMLS shows the child's prose `{{ … }}` spans as inert under
  `interpolate=false`, like interpolation literals.
- The canonical page for `::file` options
  (`darkmatter/docs/transclusion/block-transclusion.md`, "Options and
  Conditionals") documents it with an example; the `darkmatter` skill and the
  caller skill `use-darkmatter` (`darkmatter/docs/use-skills/`) list it.
- `CLAUDE.md`'s Skill Audiences rule for transcluding canonical docs into
  caller skills becomes "transclude with `interpolate=false`"; the
  publication guard (`--deny all` in `just/skills.just`) stays as a backstop.

## Verification

- A child whose prose shows `{{ name }}`, `{{ undeclared }}`, and
  `{{{ x }}}` transcludes them unchanged under `interpolate=false`, and
  evaluates them as today without it.
- The child's frontmatter interpolation still runs under `interpolate=false`.
- Nested transclusion inherits the setting; `::block when=` still selects.
- A non-boolean value is a located error.

---
area: darkmatter
status: finalized-spec
created: 2026-10-10
owner: Ken Snyder <ken@ken.net>
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
    formerly: string -> the spec's previous `{date}-{name}`, when it was renamed in the 2026-10-10 restructure
    pipeline_step: number -> position in the claudine/docs/rollout-strategy.md pipeline
reviewed: false
human_review: false
pipeline_step: 1
related:
    - 2026-10-20-lifecycle-grammar
    - 2026-10-29-recursive-schema-types
---

# Schema Loader: Indentless Sequences and Folded Plain Scalars

A hand-edited SimplifiedSchema file can be written in two ordinary YAML
spellings that every YAML parser accepts but that Darkmatter's standalone
schema loader rejects today. The whole file fails to load with
`could not project SimplifiedSchema expression spans through YAML source`, so
`md schema validate`, composition, trigger discovery, and DMLS all lose the
schema. After this change both spellings load, and every diagnostic and editor
feature that points into them (grammar errors, DMLS hover, completion,
suggestion linting) lands on the correct physical line and column.

The two spellings are common because tools produce them: PyYAML's default
dump, `yq`, and many editors write a block sequence without indenting it under
its key, and editors with soft wrap fold a long plain scalar onto a second
line. The lifecycle-grammar schema spike hit both (its Finding 11) and had to
dump with `width=inf` and indented sequences to work around them.

## Decided

Decided 2026-10-10: the schema loader accepts both spellings, with precise
source positions, and the documented accepted-YAML boundary is updated to
list them.

### 1. Indentless block sequences

A block sequence whose `-` sits at the same column as its parent mapping key
is accepted anywhere a block sequence is accepted today (a named type's union
arms, a property-level union, a nested mapping value, at any depth).

Reproduction (fails today, loads after this change):

```yaml
kind: schema
types:
  t:
  - string
  - number
```

`t` is the property-level union `[string, number]`, identical to the indented
spelling. A sequence of compact mappings written indentless (`- a: string`)
is accepted the same way.

### 2. Plain scalars folded across physical lines

A plain (unquoted) scalar that continues onto one or more more-indented lines
is accepted. Its value is the YAML-folded value (line breaks become single
spaces), exactly as `serde_yaml_ng` reads it; the type-expression grammar sees
that folded text.

Reproduction (fails today, loads after this change):

```yaml
kind: schema
types:
  t: string(min(1)) -> a description long enough that the editor
    wraps it onto a second physical line
```

`t` is `string(min(1))` with the description `a description long enough that
the editor wraps it onto a second physical line`.

### 3. Positions stay exact

Every span projected from either spelling maps back to the authored bytes. In
particular, a grammar error, suggestion-lint finding, or DMLS range inside the
continuation line of a folded scalar reports that continuation line and the
column of the offending text, not the first line of the scalar and not
`0..0`. A projection that genuinely cannot be computed still fails with the
existing typed error; it never silently reports a wrong position.

### 4. The boundary stays closed otherwise

Only these two presentations join the accepted set. Block scalars (`|`, `>`),
tags, multi-line quoted scalars, complex keys, explicit flow-mapping keys, and
directives remain outside the schema-document source-map contract, exactly as
today. The frontmatter locator, which already tolerates more presentations,
is unchanged.

## Requirements

- The strict schema-document locator in
  `darkmatter/lib/src/markdown/schemas/simplified/source.rs`
  (`locate_yaml_value` and the `BlockLocator` it drives) accepts the two
  presentations above. The tolerant frontmatter path
  (`locate_frontmatter_value`) already handles both; reuse its logic for these
  two cases rather than writing a third locator, without widening the strict
  path to the other presentations it tolerates.
- The implementer updates the closed-boundary documentation at
  `darkmatter/docs/topics/schemas/definition.md` ("Source-Aware Presentation
  Boundary", around lines 155-174) to list indentless block sequences and
  folded multi-line plain scalars as accepted presentations, with a compact
  example of each, and removes any wording that implies they are rejected.
- Update the `darkmatter` skill's schema notes if they describe the boundary.

## Out of scope

- Any other YAML presentation (see Decided 4).
- Named-type reference semantics: `2026-10-13-named-type-presence-and-local-names`.
- Diagnostic positions inside validated frontmatter documents:
  `2026-10-15-diagnostic-node-positions`.

## Acceptance criteria

1. Both reproduction files above load through
   `parse_standalone_schema_document` and through `md schema validate` run
   against a document that references them; the resolved JSON Schema is
   byte-identical to the one produced from the indented / single-line
   spelling of the same file.
2. An indentless sequence nested two levels deep (a union arm list under a
   mapping under `types`) and an indentless sequence of compact mappings both
   load and resolve identically to their indented spellings.
3. A folded plain scalar whose continuation line contains a grammar error
   reports the error with the continuation line's line number and the column
   of the offending token. Example: `t: string(min(1);` on one line and
   `    bogus(2))` on the next folds to `string(min(1); bogus(2))`; the
   unknown-constraint error points at `bogus` on the second line.
4. DMLS: hover and completion inside a folded continuation line and inside an
   indentless sequence item resolve to the same schema node as in the
   indented / single-line spelling (L1 test through the DMLS test harness).
5. Block scalars, tags, and multi-line quoted scalars in a schema document
   still fail with the existing projection error (regression tests).
6. The shipped schema corpus loads unchanged (existing corpus tests green).
7. `just test` and `just lint` pass in `darkmatter/`. Tests are L1, use no
   terminal or browser windows, and are portable to macOS, Linux, Windows, and
   WSL2 (fixture text uses `\n` and is written through the repo's fixture
   helpers, not checked-in files with platform line endings). No new CI cells.

## Open questions

None.
